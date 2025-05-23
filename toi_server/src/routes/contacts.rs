use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{Datelike, Duration, Month, NaiveDate};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        contacts::{
            Contact, ContactDeleteParams, ContactQueryParams, NewContact, NewContactRequest,
            UpdateContactRequest,
        },
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find contacts stored with details that the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn contacts_router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(
            add_contact,
            delete_matching_contacts,
            get_matching_contacts,
            update_matching_contact
        ))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update POST /contacts extensions
    let add_contact_json_schema = schema_for!(NewContactRequest);
    let add_contact_json_schema =
        serde_json::to_value(add_contact_json_schema).expect("schema unserializable");
    let add_contact_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", add_contact_json_schema)
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(add_contact_extensions);

    // Update PUT /contacts extensions
    let update_contact_json_schema = schema_for!(UpdateContactRequest);
    let update_contact_json_schema =
        serde_json::to_value(update_contact_json_schema).expect("schema unserializable");
    let update_contact_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", update_contact_json_schema)
        .build();
    paths
        .put
        .as_mut()
        .expect("PUT doesn't exist")
        .extensions
        .get_or_insert(update_contact_extensions);

    // Update DELETE and GET /contacts extensions
    let contacts_json_schema = schema_for!(ContactQueryParams);
    let contacts_json_schema =
        serde_json::to_value(contacts_json_schema).expect("schema unserializable");
    let contacts_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", contacts_json_schema)
        .build();
    paths
        .delete
        .as_mut()
        .expect("DELETE doesn't exist")
        .extensions
        .get_or_insert(contacts_extensions.clone());
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(contacts_extensions);

    router
}

pub async fn search_contacts(
    state: &ToiState,
    params: ContactQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let ContactQueryParams {
        ids,
        birthday_falls_on_search_params,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        limit,
    } = params;

    let mut query = schema::contacts::table
        .select(Contact::as_select())
        .into_boxed();

    // Filter items created on or after date.
    if let Some(created_from) = created_from {
        query = query.filter(schema::contacts::created_at.ge(created_from));
    }

    // Filter items created on or before date.
    if let Some(created_to) = created_to {
        query = query.filter(schema::contacts::created_at.le(created_to));
    }

    // Filter items according to birthdays.
    if let Some(birthday_falls_on_search_params) = birthday_falls_on_search_params {
        match birthday_falls_on_search_params.falls_on {
            utils::DateFallsOn::Month => {
                let year = birthday_falls_on_search_params.birthday.year();
                let month = birthday_falls_on_search_params.birthday.month();
                let num_days_in_month = {
                    let month = u8::try_from(month).map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "invalid birthday search month".to_string(),
                        )
                    })?;
                    let month = Month::try_from(month).map_err(|_| {
                        (
                            StatusCode::BAD_REQUEST,
                            "invalid birthday search month".to_string(),
                        )
                    })?;
                    month.num_days(year).ok_or((
                        StatusCode::BAD_REQUEST,
                        "invalid birthday search year".to_string(),
                    ))?
                };
                let first_day_of_month = NaiveDate::from_ymd_opt(year, month, 1).ok_or((
                    StatusCode::BAD_REQUEST,
                    "invalid birthday search".to_string(),
                ))?;
                let last_day_of_month =
                    NaiveDate::from_ymd_opt(year, month, num_days_in_month.into()).ok_or((
                        StatusCode::BAD_REQUEST,
                        "invalid birthday search".to_string(),
                    ))?;
                query = query.filter(
                    schema::contacts::birthday
                        .ge(first_day_of_month)
                        .and(schema::contacts::birthday.le(last_day_of_month)),
                );
            }
            utils::DateFallsOn::Week => {
                let num_days_from_sunday = birthday_falls_on_search_params
                    .birthday
                    .weekday()
                    .num_days_from_sunday();
                let this_weeks_sunday = birthday_falls_on_search_params.birthday
                    - Duration::days(num_days_from_sunday.into());
                let this_weeks_saturday = this_weeks_sunday + Duration::days(6);
                query = query.filter(
                    schema::contacts::birthday
                        .ge(this_weeks_sunday)
                        .and(schema::contacts::birthday.le(this_weeks_saturday)),
                );
            }
            utils::DateFallsOn::Day => {
                query = query.filter(
                    schema::contacts::birthday.eq(birthday_falls_on_search_params.birthday),
                );
            }
        }
    }

    // Order items.
    match order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::contacts::created_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::contacts::created_at.desc()),
        None => {
            // By default, filter items similar to a given query.
            if let Some(ref similarity_search_params) = similarity_search_params {
                let input = EmbeddingPromptTemplate::builder()
                    .instruction_prefix(INSTRUCTION_PREFIX.to_string())
                    .query_prefix(QUERY_PREFIX.to_string())
                    .build()
                    .apply(&similarity_search_params.query);
                let embedding_request = EmbeddingRequest { input };
                let embedding = state.model_client.embed(embedding_request).await?;
                query = query
                    .filter(
                        schema::contacts::embedding
                            .cosine_distance(embedding.clone())
                            .le(state.server_config.distance_threshold),
                    )
                    .order(schema::contacts::embedding.cosine_distance(embedding));
            }
        }
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        query = query.or_filter(schema::contacts::id.eq_any(ids))
    }

    // Limit number of items.
    if let Some(limit) = limit {
        query = query.limit(limit);
    }

    // Get all the items that match the query.
    let contacts: Vec<Contact> = query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) = contacts
        .into_iter()
        .map(|contact| {
            let Contact {
                id,
                first_name,
                last_name,
                email,
                phone,
                birthday,
                relationship,
                ..
            } = contact;
            let new_contact_request = NewContactRequest {
                first_name,
                last_name,
                email,
                phone,
                birthday,
                relationship,
            };
            (id, new_contact_request.to_string())
        })
        .unzip();
    if ids.is_empty() {
        return Err((StatusCode::NOT_FOUND, "no contacts found".to_string()));
    }

    // Rerank and filter items once more.
    let ids = match similarity_search_params {
        Some(similarity_search_params) => {
            if similarity_search_params.use_reranking_filter {
                let rerank_request = RerankRequest {
                    query: similarity_search_params.query,
                    documents,
                };
                let rerank_response = state.model_client.rerank(rerank_request).await?;
                rerank_response
                    .results
                    .into_iter()
                    .filter(|item| item.relevance_score >= state.server_config.similarity_threshold)
                    .map(|item| ids[item.index])
                    .collect()
            } else {
                ids
            }
        }
        None => ids,
    };

    Ok(ids)
}

/// Add and return a contact.
///
/// Example queries for adding contacts using this endpoint:
/// - Add a contact
/// - Remember this contact
/// - Make a contact
#[utoipa::path(
    post,
    path = "",
    request_body = NewContactRequest,
    responses(
        (status = 201, description = "Successfully added a contact", body = Contact),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_contact(
    State(state): State<ToiState>,
    Json(body): Json<NewContactRequest>,
) -> Result<Json<Contact>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let embedding_request = EmbeddingRequest {
        input: body.to_string(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let NewContactRequest {
        first_name,
        last_name,
        email,
        phone,
        birthday,
        relationship,
    } = body;
    let new_contact = NewContact {
        first_name,
        last_name,
        email,
        phone,
        birthday,
        relationship,
        embedding,
    };
    let res = diesel::insert_into(schema::contacts::table)
        .values(new_contact)
        .returning(Contact::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Delete and return contacts.
///
/// Example queries for deleting contacts using this endpoint:
/// - Delete all contacts
/// - Erase all contacts
/// - Remove contacts
#[utoipa::path(
    delete,
    path = "",
    params(ContactDeleteParams),
    responses(
        (status = 200, description = "Successfully deleted contacts", body = [Contact]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No contacts found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_contacts(
    State(state): State<ToiState>,
    Query(params): Query<ContactDeleteParams>,
) -> Result<Json<Vec<Contact>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ContactDeleteParams {
        ids,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        limit,
    } = params;
    let params = ContactQueryParams {
        ids,
        birthday_falls_on_search_params: None,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        limit,
    };
    let ids = search_contacts(&state, params, &mut conn).await?;
    let contacts = diesel::delete(schema::contacts::table.filter(schema::contacts::id.eq_any(ids)))
        .returning(Contact::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(contacts))
}

/// Get contacts.
///
/// Example queries for getting contacts using this endpoint:
/// - Get all contacts
/// - List all contacts
/// - What contacts
/// - How many contacts
#[utoipa::path(
    get,
    path = "",
    params(ContactQueryParams),
    responses(
        (status = 200, description = "Successfully got contacts", body = [Contact]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No contacts found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_contacts(
    State(state): State<ToiState>,
    Query(params): Query<ContactQueryParams>,
) -> Result<Json<Vec<Contact>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_contacts(&state, params, &mut conn).await?;
    let contacts = schema::contacts::table
        .select(Contact::as_select())
        .filter(schema::contacts::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(contacts))
}

/// Update and return a contact.
///
/// Example queries for updating a contact using this endpoint:
/// - Update my contact
/// - Update the birthday
/// - Update the email
/// - An update on my relationship
#[utoipa::path(
    put,
    path = "",
    request_body = UpdateContactRequest,
    responses(
        (status = 200, description = "Successfully updated contact", body = Contact),
        (status = 404, description = "Contact not found")
    )
)]
#[axum::debug_handler]
pub async fn update_matching_contact(
    State(state): State<ToiState>,
    Json(body): Json<UpdateContactRequest>,
) -> Result<Json<Contact>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let UpdateContactRequest {
        id,
        contact_updates,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
    } = body;
    let params = ContactQueryParams {
        ids: id.map(|i| vec![i]),
        birthday_falls_on_search_params: None,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        limit: Some(1),
    };
    let id = search_contacts(&state, params, &mut conn)
        .await?
        .into_iter()
        .next()
        .ok_or((StatusCode::NOT_FOUND, "contact not found".to_string()))?;
    let mut contact = schema::contacts::table
        .select(Contact::as_select())
        .filter(schema::contacts::id.eq(id))
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    contact.update(contact_updates);
    let Contact {
        id,
        first_name,
        last_name,
        email,
        phone,
        birthday,
        relationship,
        ..
    } = contact;
    let new_contact_request = NewContactRequest {
        first_name,
        last_name,
        email,
        phone,
        birthday,
        relationship,
    };
    let embedding_request = EmbeddingRequest {
        input: new_contact_request.to_string(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let NewContactRequest {
        first_name,
        last_name,
        email,
        phone,
        birthday,
        relationship,
    } = new_contact_request;
    let new_contact = NewContact {
        first_name,
        last_name,
        email,
        phone,
        birthday,
        relationship,
        embedding,
    };
    let contact = diesel::update(schema::contacts::table.filter(schema::contacts::id.eq(id)))
        .set(&new_contact)
        .returning(Contact::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(contact))
}
