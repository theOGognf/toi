use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Json, Redirect},
};
use chrono::{Duration, Utc};
use diesel::{ExpressionMethods, PgSortExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use rand::seq::SliceRandom;
use schemars::schema_for;
use tracing::debug;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    models::{
        client::ApiClientError,
        news::{Alias, ExpiredRedirect, GetNewsRequest, NewAlias, NewRedirect, News},
        state::ToiState,
    },
    schema, utils,
};

const ALIASES: &str = include_str!("../../data/aliases.txt");

pub async fn news_router(state: ToiState) -> Result<OpenApiRouter, Box<dyn std::error::Error>> {
    let mut new_aliases: Vec<String> = ALIASES
        .lines()
        .filter_map(|item| {
            if item.is_empty() {
                None
            } else {
                Some(item.to_string())
            }
        })
        .collect();
    new_aliases.sort();
    new_aliases.dedup();
    new_aliases.shuffle(&mut rand::rng());
    let server_addr = format!("127.0.0.1:{}", &state.server_config.bind_addr.port());
    let new_aliases: Vec<NewAlias> = new_aliases
        .into_iter()
        .map(|alias| NewAlias::new(&server_addr, alias))
        .collect();
    let mut conn = state.pool.get().await?;
    diesel::delete(schema::news::table)
        .execute(&mut conn)
        .await?;
    diesel::insert_into(schema::news::table)
        .values(&new_aliases)
        .execute(&mut conn)
        .await?;

    drop(conn);

    let router = OpenApiRouter::new()
        .routes(routes!(get_news_article, get_news))
        .with_state(state);

    Ok(router)
}

/// Get a specific news article by its alias.
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
    let aliases: Vec<String> = schema::news::table
        .select(schema::news::alias)
        .filter(schema::news::updated_at.lt(cutoff))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    if !aliases.is_empty() {
        diesel::update(schema::news::table.filter(schema::news::alias.eq_any(aliases)))
            .set(ExpiredRedirect::default())
            .execute(&mut conn)
            .await
            .map_err(utils::diesel_error)?;
    }
    let url: Option<String> = schema::news::table
        .select(schema::news::url)
        .filter(schema::news::alias.eq(alias))
        .first(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    match url {
        Some(url) => Ok(Redirect::temporary(&url)),
        None => Err((StatusCode::NOT_FOUND, "news article not found".to_string())),
    }
}

/// Get news, returning news article titles with the links to the articles together.
///
/// Example queries for getting news using this endpoint:
/// - Get news from apnews.com.
/// - Get news from the past 10 hours.
/// - Show me good news.
#[utoipa::path(
    post,
    path = "",
    extensions(
        ("x-json-schema-body" = json!(schema_for!(GetNewsRequest)))
    ),
    request_body = GetNewsRequest,
    responses(
        (status = 201, description = "Successfully got news", body = [NewRedirect]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_news(
    State(state): State<ToiState>,
    Json(body): Json<GetNewsRequest>,
) -> Result<Json<Vec<NewRedirect>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    // First, expire old links.
    let cutoff = Utc::now() - Duration::hours(24);
    let aliases: Vec<String> = schema::news::table
        .select(schema::news::alias)
        .filter(schema::news::updated_at.lt(cutoff))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    if !aliases.is_empty() {
        diesel::update(schema::news::table.filter(schema::news::alias.eq_any(aliases)))
            .set(ExpiredRedirect::default())
            .execute(&mut conn)
            .await
            .map_err(utils::diesel_error)?;
    }
    // Get the RSS query from the body.
    let (url, params) = body.into();
    debug!("getting rss feed with {params:?}");
    // Get RSS items from the feed.
    let content = state
        .api_client
        .get(url)
        .query(&params)
        .send()
        .await
        .map_err(|err| ApiClientError::ApiConnection.into_response(&err))?
        .bytes()
        .await
        .map_err(|err| ApiClientError::ApiConnection.into_response(&err))?;
    let channel = rss::Channel::read_from(&content[..]).map_err(utils::internal_error)?;
    let items: Vec<rss::Item> = channel
        .items
        .into_iter()
        .filter(|item| item.title.is_some() && item.link.is_some())
        .collect();
    debug!("got {} news items", items.len());
    // Convert the items into redirects that're sent to the client.
    let redirects = conn
        .transaction(|mut conn| {
            async move {
                // Get all the aliases used as redirects for the RSS items.
                let aliases: Vec<String> = schema::news::table
                    .select(schema::news::alias)
                    .order_by(schema::news::updated_at.asc().nulls_first())
                    .limit(
                        items
                            .len()
                            .try_into()
                            .expect("news items length should fit in i64"),
                    )
                    .load(&mut conn)
                    .await?;
                // Delete all the selected aliases. Have to do this because we
                // can't batch update. Instead of batch updating, we batch delete
                // and then batch insert in one transaction.
                let aliases = diesel::delete(
                    schema::news::table.filter(schema::news::alias.eq_any(&aliases)),
                )
                .returning(Alias::as_returning())
                .load(&mut conn)
                .await?;
                // Insert the new news items, filling back in the deleted aliases.
                let news: Vec<News> = aliases
                    .into_iter()
                    .zip(items.into_iter())
                    .map(|(alias, item)| News {
                        alias: alias.alias,
                        tinyurl: alias.tinyurl,
                        url: item.link,
                        title: item.title,
                        updated_at: Some(Utc::now()),
                    })
                    .collect();
                let redirects = diesel::insert_into(schema::news::table)
                    .values(news)
                    .returning(NewRedirect::as_returning())
                    .load(&mut conn)
                    .await?;
                Ok(redirects)
            }
            .scope_boxed()
        })
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(redirects))
}
