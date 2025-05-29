use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::{
    recipes::{NewRecipeRequest, Recipe, RecipeSearchParams, RecipeTagSearchParams, RecipeTags},
    tags::{NewTagRequest, Tag},
};

mod utils;

#[tokio::test]
#[serial]
async fn recipes_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    utils::reset_database(&db_connection_url)?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router = OpenApiRouter::new()
        .nest(
            "/recipes",
            toi_server::routes::recipes::recipes_router(state.clone()),
        )
        .nest(
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

    // Make a recipe.
    let recipes_url = format!("http://{}/recipes", state.server_config.bind_addr);
    let description = "steamed jasmine rice".to_string();
    let tags = vec![name1];
    let body = NewRecipeRequest::builder()
        .description(description.clone())
        .ingredients("jasmine rice".to_string())
        .instructions("1. wash rice, 2. add 1:1 water:rice in a rice cooker, 3. turn on cooker, 4. fluff and serve when done".to_string())
        .tags(tags.clone())
        .build();
    let response = client.post(&recipes_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let recipe1 = response.json::<Recipe>().await?;
    assert_eq!(recipe1.description, description);

    // Retrieve the recipe tag using search.
    let search_recipe_tags_url = format!("{recipes_url}/tags/search");
    let params = RecipeTagSearchParams::builder()
        .recipe_query("rice".to_string())
        .tag_query("asian".to_string())
        .build();
    let response = client
        .post(search_recipe_tags_url)
        .json(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let recipe_tags1 = response.json::<RecipeTags>().await?;
    assert_eq!(recipe_tags1.tags, vec![tag1]);

    // Retrieve the recipe using search.
    let search_recipes_url = format!("{recipes_url}/search");
    let params = RecipeSearchParams::builder()
        .query("rice".to_string())
        .tags(tags)
        .build();
    let response = client.post(search_recipes_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_recipes1 = response.json::<Vec<Recipe>>().await?;
    assert_eq!(vec_recipes1, vec![recipe1]);

    // Delete the recipe using search.
    let delete_recipes_url = format!("{recipes_url}/delete");
    let response = client.post(delete_recipes_url).json(&params).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_recipes2 = response.json::<Vec<Recipe>>().await?;
    assert_eq!(vec_recipes2, vec_recipes1);
    Ok(())
}
