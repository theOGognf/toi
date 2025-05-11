use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Redirect},
};
use chrono::{Duration, Utc};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use schemars::schema_for;
use std::io::BufReader;
use std::{fs, io::BufRead};
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        news::{NewNews, NewNewsRequest, News, NewsQueryParams},
        state::ToiState,
    },
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find news headlines similar to the one the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub async fn router(state: ToiState) -> Result<OpenApiRouter, Box<dyn std::error::Error>> {
    let file = fs::File::open("names.txt")?;
    let new_news_items: Vec<NewNews> = BufReader::new(file)
        .lines()
        .flatten()
        .map(|name| NewNews::new(name))
        .collect();
    let mut conn = state.pool.get().await?;
    diesel::delete(schema::news::table)
        .execute(&mut conn)
        .await?;
    diesel::insert_into(schema::news::table)
        .values(&new_news_items)
        .execute(&mut conn)
        .await?;
    let mut router = OpenApiRouter::new()
        .routes(routes!(get_news_article, update_news,))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update POST /news extensions
    let update_news_json_schema = schema_for!(NewNewsRequest);
    let update_news_json_schema =
        serde_json::to_value(update_news_json_schema).expect("schema unserializable");
    let update_news_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-body", update_news_json_schema)
        .build();
    paths
        .post
        .as_mut()
        .expect("POST doesn't exist")
        .extensions
        .get_or_insert(update_news_extensions);

    Ok(router)
}

/// Get news.
///
/// Example queries for getting news using this endpoint:
/// - Get all news related to...
/// - List all news about...
/// - What news are there on...
/// - How many news are there about...
#[utoipa::path(
    get,
    path = "/{alias}",
    params(
        ("alias" = String, Path, description = "Alias for news article URL")
    )
)]
#[axum::debug_handler]
pub async fn get_news_article(
    State(state): State<ToiState>,
    Path(alias): Path<String>,
) -> Result<Redirect, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let cutoff = Utc::now() - Duration::hours(24);
    diesel::delete(schema::news::table.filter(schema::news::updated_at.lt(cutoff)))
        .execute(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let url: Option<String> = schema::news::table
        .select(schema::news::url)
        .filter(schema::news::alias.eq(alias))
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    match url {
        Some(url) => Ok(Redirect::temporary(&url)),
        None => Err((StatusCode::NOT_FOUND, "news article not found".to_string())),
    }
}

/// Add and return a new.
///
/// Example queries for adding news using this endpoint:
/// - Add a new saying...
/// - Add a new that...
/// - Keep new on...
/// - Remember that...
/// - Make a new...
#[utoipa::path(
    post,
    path = "",
    request_body = UpdateNewsRequest,
    responses(
        (status = 201, description = "Successfully added a new", body = News),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn update_news(
    State(state): State<ToiState>,
    Json(body): Json<NewNewsRequest>,
) -> Result<Json<News>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let cutoff = Utc::now() - Duration::hours(24);
    diesel::delete(schema::news::table.filter(schema::news::updated_at.lt(cutoff)))
        .execute(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    // Get results using quick_xml serde feature
    // Update table, returning the localhost URL for the redirect and the article description for all results
    let todos = diesel::update(schema::todos::table.filter(schema::todos::id.eq_any(ids)))
        .set(schema::todos::completed_at.eq(completed_at))
        .returning(Todo::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(res))
}
