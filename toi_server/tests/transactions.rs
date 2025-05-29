use serial_test::serial;
use tokio::net::TcpListener;
use utoipa_axum::router::OpenApiRouter;

use toi_server::models::{
    accounts::{BankAccount, NewBankAccountRequest},
    transactions::{
        BankAccountHistory, BankAccountTransaction, BankAccountTransactionSearchParams,
        NewBankAccountTransactionRequest,
    },
};

mod utils;

#[tokio::test]
#[serial]
async fn transactions_routes() -> Result<(), Box<dyn std::error::Error>> {
    // Make sure there's a database URL and it points to a test database so
    // prod isn't goofed during testing.
    let db_connection_url = dotenvy::var("DATABASE_URL")?;
    utils::reset_database(&db_connection_url)?;

    // Initialize the server state.
    let state = toi_server::init(db_connection_url).await?;
    let openapi_router = OpenApiRouter::new().nest(
        "/banking/accounts",
        toi_server::routes::accounts::accounts_router(state.clone()).nest(
            "/transactions",
            toi_server::routes::transactions::bank_account_transactions_router(state.clone()),
        ),
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

    // Make a transaction.
    let bank_account_transactions_url = format!("{accounts_url}/transactions");
    let transaction_description = "coffee shop".to_string();
    let transaction_amount = 4.20;
    let body = NewBankAccountTransactionRequest::builder()
        .bank_account_query(account_description.clone())
        .transaction_description(transaction_description.clone())
        .transaction_amount(transaction_amount)
        .build();
    let response = client
        .post(&bank_account_transactions_url)
        .json(&body)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let bank_account_transaction1 = response.json::<BankAccountTransaction>().await?;
    assert_eq!(bank_account_transaction1.bank_account, account1);

    // Retrieve the transaction using search.
    let search_bank_account_transactions_url = format!("{bank_account_transactions_url}/search");
    let params = BankAccountTransactionSearchParams::builder()
        .bank_account_query(account_description.clone())
        .transaction_query(transaction_description.clone())
        .build();
    let response = client
        .post(search_bank_account_transactions_url)
        .json(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let bank_account_history1 = response.json::<BankAccountHistory>().await?;
    assert_eq!(
        bank_account_history1.transactions,
        vec![bank_account_transaction1.transaction]
    );

    // Delete the transaction using search.
    let delete_bank_account_transactions_url = format!("{bank_account_transactions_url}/delete");
    let response = client
        .post(delete_bank_account_transactions_url)
        .json(&params)
        .send()
        .await?;
    let response = utils::assert_ok_response(response).await?;
    let bank_account_history2 = response.json::<BankAccountHistory>().await?;
    assert_eq!(bank_account_history2, bank_account_history1);
    Ok(())
}
