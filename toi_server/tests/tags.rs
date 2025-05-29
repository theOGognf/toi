use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::tags::{NewTagRequest, Tag, TagSearchParams};

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
    let tags_url = format!("http://{}/tags", state.server_config.bind_addr);

    // Make a tag.
    let name1 = "asian".to_string();
    let body = NewTagRequest::builder().name(name1.clone()).build();
    let response = client.post(&tags_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let tag1 = response.json::<Tag>().await?;
    assert_eq!(tag1.name, name1);

    // Make a second tag.
    let name2 = "korean".to_string();
    let body = NewTagRequest::builder().name(name2.clone()).build();
    let response = client.post(&tags_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let tag2 = response.json::<Tag>().await?;
    assert_eq!(tag2.name, name2);

    // Retrieve the second tag using search.
    let search_tags_url = format!("{tags_url}/search");
    let params = TagSearchParams::builder()
        .query("korean".to_string())
        .use_reranking_filter(true)
        .use_edit_distance_filter(true)
        .build();
    let response = client.post(search_tags_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_tags1 = response.json::<Vec<Tag>>().await?;
    assert_eq!(vec_tags1, vec![tag2]);

    // Delete the tag using search.
    let delete_tags_url = format!("{tags_url}/delete");
    let response = client.post(delete_tags_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_tags2 = response.json::<Vec<Tag>>().await?;
    assert_eq!(vec_tags2, vec_tags1);
    Ok(())
}
