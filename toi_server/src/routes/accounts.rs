use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use pgvector::VectorExpressionMethods;
use schemars::schema_for;
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        accounts::{BankAccount, BankAccountQueryParams, NewBankAccount, NewBankAccountRequest},
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str = "Instruction: Given a user query, find bank accounts stored with details that the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(
            add_bank_account,
            delete_matching_bank_accounts,
            get_matching_bank_accounts,
        ))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update POST /banking/accounts extensions
    let add_bank_account_json_schema = schema_for!(NewBankAccountRequest);
    let add_bank_account_json_schema =
        serde_json::to_value(add_bank_account_json_schema).expect("schema unserializable");
    let add_bank_account_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", add_bank_account_json_schema)
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(add_bank_account_extensions);

    // Update DELETE and GET /banking/accounts extensions
    let bank_accounts_json_schema = schema_for!(BankAccountQueryParams);
    let bank_accounts_json_schema =
        serde_json::to_value(bank_accounts_json_schema).expect("schema unserializable");
    let bank_accounts_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", bank_accounts_json_schema)
        .build();
    paths
        .delete
        .as_mut()
        .expect("DELETE doesn't exist")
        .extensions
        .get_or_insert(bank_accounts_extensions.clone());
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(bank_accounts_extensions);

    router
}

pub async fn search_bank_accounts(
    state: &ToiState,
    params: BankAccountQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let BankAccountQueryParams {
        ids,
        similarity_search_params,
        created_from,
        created_to,
        order_by,
        limit,
    } = params;

    let mut query = schema::bank_accounts::table
        .select(BankAccount::as_select())
        .into_boxed();

    // Filter items created on or after date.
    if let Some(created_from) = created_from {
        query = query.filter(schema::bank_accounts::created_at.ge(created_from));
    }

    // Filter items created on or before date.
    if let Some(created_to) = created_to {
        query = query.filter(schema::bank_accounts::created_at.le(created_to));
    }

    // Order items.
    match order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::bank_accounts::created_at),
        Some(utils::OrderBy::Newest) => {
            query = query.order(schema::bank_accounts::created_at.desc())
        }
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
                        schema::bank_accounts::embedding
                            .cosine_distance(embedding.clone())
                            .le(state.server_config.distance_threshold),
                    )
                    .order(schema::bank_accounts::embedding.cosine_distance(embedding));
            }
        }
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        query = query.or_filter(schema::bank_accounts::id.eq_any(ids))
    }

    // Limit number of items.
    if let Some(limit) = limit {
        query = query.limit(limit);
    }

    // Get all the items that match the query.
    let bank_accounts: Vec<BankAccount> = query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) = bank_accounts
        .into_iter()
        .map(|bank_account| (bank_account.id, bank_account.description))
        .unzip();
    if ids.is_empty() {
        return Err((StatusCode::NOT_FOUND, "no bank accounts found".to_string()));
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

/// Add and return a bank account.
///
/// Example queries for adding a bank account using this endpoint:
/// - Add a bank account with
/// - Remember this bank account
/// - Make a bank account
#[utoipa::path(
    post,
    path = "",
    request_body = NewBankAccountRequest,
    responses(
        (status = 201, description = "Successfully added a bank account", body = BankAccount),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_bank_account(
    State(state): State<ToiState>,
    Json(body): Json<NewBankAccountRequest>,
) -> Result<Json<BankAccount>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let embedding_request = EmbeddingRequest {
        input: body.description.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let NewBankAccountRequest { description } = body;
    let new_bank_account = NewBankAccount {
        description,
        embedding,
    };
    let res = diesel::insert_into(schema::bank_accounts::table)
        .values(new_bank_account)
        .returning(BankAccount::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}

/// Delete and return bank accounts.
///
/// Example queries for deleting bank accounts using this endpoint:
/// - Delete all bank accounts with
/// - Erase all bank accounts that
/// - Remove bank accounts with
/// - Delete bank accounts
#[utoipa::path(
    delete,
    path = "",
    params(BankAccountQueryParams),
    responses(
        (status = 200, description = "Successfully deleted bank accounts", body = [BankAccount]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No bank accounts found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_bank_accounts(
    State(state): State<ToiState>,
    Query(params): Query<BankAccountQueryParams>,
) -> Result<Json<Vec<BankAccount>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_bank_accounts(&state, params, &mut conn).await?;
    let bank_accounts =
        diesel::delete(schema::bank_accounts::table.filter(schema::bank_accounts::id.eq_any(ids)))
            .returning(BankAccount::as_returning())
            .load(&mut conn)
            .await
            .map_err(utils::diesel_error)?;
    Ok(Json(bank_accounts))
}

/// Get bank accounts.
///
/// Example queries for getting bank accounts using this endpoint:
/// - Get all bank accounts where
/// - List all bank accounts
/// - What bank accounts do I have on
/// - How many bank accounts do I have
#[utoipa::path(
    get,
    path = "",
    params(BankAccountQueryParams),
    responses(
        (status = 200, description = "Successfully got bank accounts", body = [BankAccount]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No bank accounts found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_bank_accounts(
    State(state): State<ToiState>,
    Query(params): Query<BankAccountQueryParams>,
) -> Result<Json<Vec<BankAccount>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let ids = search_bank_accounts(&state, params, &mut conn).await?;
    let bank_accounts = schema::bank_accounts::table
        .select(BankAccount::as_select())
        .filter(schema::bank_accounts::id.eq_any(ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(bank_accounts))
}
