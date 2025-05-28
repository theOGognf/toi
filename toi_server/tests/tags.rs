use serde_json::json;
use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::tags::{Tag, TagQueryParams};

mod utils;

#[tokio::test]
#[serial]
async fn tags_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    utils::reset_database(&db_connection_url)?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router = OpenApiRouter::new().nest(
        "/tags",
        toi_server::routes::tags::tags_router(state.clone()),
    );
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let url = format!("http://{}/tags", state.server_config.bind_addr);

    // Make a tag.
    let name1 = "asian";
    let body = json!(
        {
            "name": name1
        }
    );
    let response = client.post(&url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let tag1 = response.json::<Tag>().await?;
    assert_eq!(tag1.name, name1);

    // Make a second tag.
    let name2 = "korean";
    let body = json!(
        {
            "name": name2
        }
    );
    let response = client.post(&url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let tag2 = response.json::<Tag>().await?;
    assert_eq!(tag2.name, name2);

    // Retrieve the second tag using search.
    let query = TagQueryParams::builder()
        .query("korean".to_string())
        .use_reranking_filter(true)
        .use_edit_distance_filter(true)
        .build();
    let response = client.get(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_tags1 = response.json::<Vec<Tag>>().await?;
    assert_eq!(vec_tags1.len(), 1);
    assert_eq!(vec_tags1.first(), Some(tag2).as_ref());

    // Delete the tag using search.
    let response = client.delete(&url).query(&query).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_tags2 = response.json::<Vec<Tag>>().await?;
    assert_eq!(vec_tags1, vec_tags2);
    Ok(())
}
