use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::accounts::{BankAccount, BankAccountSearchParams, NewBankAccountRequest};

mod utils;

#[tokio::test]
#[serial]
async fn accounts_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    utils::reset_database(&db_connection_url)?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router = OpenApiRouter::new().nest(
        "/banking/accounts",
        toi_server::routes::accounts::accounts_router(state.clone()),
    );
    let (router, _) = openapi_router.split_for_parts();
    let listener = TcpListener::bind(&state.server_config.bind_addr).await?;

    // Spawn server and create a client for all test requests.
    let _ = tokio::spawn(async move { axum::serve(listener, router).await });
    let client = reqwest::Client::new();
    let accounts_url = format!("http://{}/banking/accounts", state.server_config.bind_addr);

    // Make an account.
    let account_description = "checking".to_string();
    let body = NewBankAccountRequest::builder()
        .description(account_description.clone())
        .build();
    let response = client.post(&accounts_url).json(&body).send().await?;
    let response = utils::assert_ok_response(response).await?;
    let account1 = response.json::<BankAccount>().await?;
    assert_eq!(account1.description, account_description);

    // Retrieve the account using search.
    let search_accounts_url = format!("{accounts_url}/search");
    let params = BankAccountSearchParams::builder()
        .query(account_description.to_string())
        .build();
    let response = client
        .post(search_accounts_url)
        .json(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_accounts1 = response.json::<Vec<BankAccount>>().await?;
    assert_eq!(vec_accounts1, vec![account1]);

    // Delete the account using search.
    let delete_accounts_url = format!("{accounts_url}/delete");
    let response = client
        .post(delete_accounts_url)
        .json(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let vec_accounts2 = response.json::<Vec<BankAccount>>().await?;
    assert_eq!(vec_accounts2, vec_accounts1);
    Ok(())
}
