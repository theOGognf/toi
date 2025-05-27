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
        accounts::{BankAccount, BankAccountQueryParams},
        client::{EmbeddingPromptTemplate, EmbeddingRequest, RerankRequest},
        search::SimilaritySearchParams,
        state::ToiState,
        transactions::{
            BankAccountHistory, BankAccountTransaction, BankAccountTransactionQueryParams,
            LinkedTransaction, NewBankAccountTransactionRequest, NewLinkedTransaction, Transaction,
            TransactionQueryParams,
        },
    },
    routes::accounts::search_bank_accounts,
    schema, utils,
};

// Prefixes are used for embedding instructions.
const INSTRUCTION_PREFIX: &str =
    "Instruction: Given a user query, find transactions stored with details that the user mentions";
const QUERY_PREFIX: &str = "Query: ";

pub fn bank_account_transactions_router(state: ToiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(
            add_bank_account_transaction,
            delete_matching_bank_account_transactions,
            get_matching_bank_account_transactions,
        ))
        .with_state(state)
}

pub fn transactions_router(state: ToiState) -> OpenApiRouter {
    let mut router = OpenApiRouter::new()
        .routes(routes!(
            delete_matching_transactions,
            get_matching_transactions,
        ))
        .with_state(state);

    let openapi = router.get_openapi_mut();
    let paths = openapi.paths.paths.get_mut("").expect("doesn't exist");

    // Update DELETE and GET /banking/transactions extensions
    let transactions_json_schema = schema_for!(TransactionQueryParams);
    let transactions_json_schema =
        serde_json::to_value(transactions_json_schema).expect("schema unserializable");
    let transaction_extensions = ExtensionsBuilder::new()
        .add("x-json-schema-params", transactions_json_schema)
        .build();
    paths
        .delete
        .as_mut()
        .expect("DELETE doesn't exist")
        .extensions
        .get_or_insert(transaction_extensions.clone());
    paths
        .get
        .as_mut()
        .expect("GET doesn't exist")
        .extensions
        .get_or_insert(transaction_extensions);

    router
}

pub async fn search_bank_account_transactions(
    state: &ToiState,
    params: BankAccountTransactionQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<(BankAccount, Vec<i32>), (StatusCode, String)> {
    let BankAccountTransactionQueryParams {
        bank_account_id,
        bank_account_query,
        bank_account_use_reranking_filter,
        bank_account_created_from,
        bank_account_created_to,
        bank_account_order_by,
        transaction_ids,
        transaction_query,
        transaction_use_reranking_filter,
        transaction_posted_from,
        transaction_posted_to,
        transaction_order_by,
        transaction_limit,
    } = params;
    let bank_account_query_params = BankAccountQueryParams {
        ids: bank_account_id.map(|i| vec![i]),
        similarity_search_params: bank_account_query.map(|query| SimilaritySearchParams {
            query,
            use_reranking_filter: bank_account_use_reranking_filter,
        }),
        created_from: bank_account_created_from,
        created_to: bank_account_created_to,
        order_by: bank_account_order_by,
        limit: Some(1),
    };
    let bank_account_id = search_bank_accounts(state, bank_account_query_params, conn)
        .await?
        .into_iter()
        .next()
        .ok_or((StatusCode::NOT_FOUND, "bank account not found".to_string()))?;
    let bank_account = schema::bank_accounts::table
        .select(BankAccount::as_select())
        .filter(schema::bank_accounts::id.eq(bank_account_id))
        .first(conn)
        .await
        .map_err(utils::diesel_error)?;

    let transaction_query_params = TransactionQueryParams {
        bank_account_id: Some(bank_account.id),
        ids: transaction_ids,
        similarity_search_params: transaction_query.map(|query| SimilaritySearchParams {
            query,
            use_reranking_filter: transaction_use_reranking_filter,
        }),
        posted_from: transaction_posted_from,
        posted_to: transaction_posted_to,
        order_by: transaction_order_by,
        limit: transaction_limit,
    };
    let transaction_ids = search_transactions(state, transaction_query_params, conn).await?;
    Ok((bank_account, transaction_ids))
}

pub async fn search_transactions(
    state: &ToiState,
    params: TransactionQueryParams,
    conn: &mut utils::Conn<'_>,
) -> Result<Vec<i32>, (StatusCode, String)> {
    let TransactionQueryParams {
        bank_account_id,
        ids,
        similarity_search_params,
        posted_from,
        posted_to,
        order_by,
        limit,
    } = params;

    let mut query = schema::transactions::table
        .select(Transaction::as_select())
        .into_boxed();

    // Filter items based on parent ID.
    if let Some(bank_account_id) = bank_account_id {
        query = query.filter(schema::transactions::bank_account_id.eq(bank_account_id));
    }

    // Filter items created on or after date.
    if let Some(posted_from) = posted_from {
        query = query.filter(schema::transactions::posted_at.ge(posted_from));
    }

    // Filter items created on or before date.
    if let Some(posted_to) = posted_to {
        query = query.filter(schema::transactions::posted_at.le(posted_to));
    }

    // Order items.
    match order_by {
        Some(utils::OrderBy::Oldest) => query = query.order(schema::transactions::posted_at),
        Some(utils::OrderBy::Newest) => query = query.order(schema::transactions::posted_at.desc()),
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
                        schema::transactions::embedding
                            .cosine_distance(embedding.clone())
                            .le(state.server_config.distance_threshold),
                    )
                    .order(schema::transactions::embedding.cosine_distance(embedding));
            }
        }
    }

    // Filter items according to their ids.
    if let Some(ids) = ids {
        query = query.or_filter(schema::transactions::id.eq_any(ids))
    }

    // Limit number of items.
    if let Some(limit) = limit {
        query = query.limit(limit);
    }

    // Get all the items that match the query.
    let transactions: Vec<Transaction> = query.load(conn).await.map_err(utils::diesel_error)?;
    let (ids, documents): (Vec<i32>, Vec<String>) = transactions
        .into_iter()
        .map(|transaction| (transaction.id, transaction.description))
        .unzip();
    if ids.is_empty() {
        return Err((StatusCode::NOT_FOUND, "no transactions found".to_string()));
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

/// Add and return a bank account transaction.
///
/// Example queries for adding bank account transactions using this endpoint:
/// - Add a bank account transaction to
/// - Remember this bank account transaction for
/// - Make a bank account transaction for
#[utoipa::path(
    post,
    path = "",
    request_body = NewBankAccountTransactionRequest,
    responses(
        (status = 201, description = "Successfully added a transaction", body = BankAccountTransaction),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn add_bank_account_transaction(
    State(state): State<ToiState>,
    Json(body): Json<NewBankAccountTransactionRequest>,
) -> Result<Json<BankAccountTransaction>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let NewBankAccountTransactionRequest {
        bank_account_id,
        bank_account_query,
        bank_account_use_reranking_filter,
        bank_account_created_from,
        bank_account_created_to,
        bank_account_order_by,
        transaction_description,
        transaction_amount,
        transaction_posted_at,
    } = body;
    let bank_account_query_params = BankAccountQueryParams {
        ids: bank_account_id.map(|i| vec![i]),
        similarity_search_params: bank_account_query.map(|query| SimilaritySearchParams {
            query,
            use_reranking_filter: bank_account_use_reranking_filter,
        }),
        created_from: bank_account_created_from,
        created_to: bank_account_created_to,
        order_by: bank_account_order_by,
        limit: Some(1),
    };
    let bank_account_id = search_bank_accounts(&state, bank_account_query_params, &mut conn)
        .await?
        .into_iter()
        .next()
        .ok_or((StatusCode::NOT_FOUND, "bank account not found".to_string()))?;
    let bank_account = schema::bank_accounts::table
        .select(BankAccount::as_select())
        .filter(schema::bank_accounts::id.eq(bank_account_id))
        .first(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let embedding_request = EmbeddingRequest {
        input: transaction_description.clone(),
    };
    let embedding = state.model_client.embed(embedding_request).await?;
    let new_transaction = NewLinkedTransaction {
        bank_account_id: bank_account.id,
        description: transaction_description,
        amount: transaction_amount,
        embedding,
        posted_at: transaction_posted_at,
    };
    let transaction = diesel::insert_into(schema::transactions::table)
        .values(new_transaction)
        .returning(Transaction::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let bank_account_transaction = BankAccountTransaction {
        bank_account,
        transaction,
    };
    Ok(Json(bank_account_transaction))
}

/// Delete and return bank account transactions.
///
/// Example queries for deleting bank account transactions using this endpoint:
/// - Delete all bank account transactions with
/// - Erase all bank account transactions for
/// - Remove bank account transactions for
/// - Delete bank account transactions
#[utoipa::path(
    delete,
    path = "",
    params(BankAccountTransactionQueryParams),
    responses(
        (status = 200, description = "Successfully deleted transactions", body = BankAccountHistory),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No bank account or transactions found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_bank_account_transactions(
    State(state): State<ToiState>,
    Query(params): Query<BankAccountTransactionQueryParams>,
) -> Result<Json<BankAccountHistory>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (bank_account, transaction_ids) =
        search_bank_account_transactions(&state, params, &mut conn).await?;
    let transactions = diesel::delete(schema::transactions::table)
        .filter(schema::transactions::id.eq_any(transaction_ids))
        .returning(Transaction::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let bank_account_history = BankAccountHistory {
        bank_account,
        transactions,
    };
    Ok(Json(bank_account_history))
}

/// Delete and return transactions.
///
/// Example queries for deleting transactions using this endpoint:
/// - Delete all transactions with
/// - Erase all transactions for
/// - Remove transactions for
/// - Delete transactions
#[utoipa::path(
    delete,
    path = "",
    params(TransactionQueryParams),
    responses(
        (status = 200, description = "Successfully deleted transactions", body = [LinkedTransaction]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No transactions found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn delete_matching_transactions(
    State(state): State<ToiState>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<Vec<LinkedTransaction>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let transaction_ids = search_transactions(&state, params, &mut conn).await?;
    let linked_transactions = diesel::delete(schema::transactions::table)
        .filter(schema::transactions::id.eq_any(transaction_ids))
        .returning(LinkedTransaction::as_returning())
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(linked_transactions))
}

/// Get bank account transactions.
///
/// Example queries for getting bank account transactions using this endpoint:
/// - Get all bank account transactions where
/// - List all bank account transactions for
/// - What bank account transactions do I have
/// - How many bank account transactions do I have
#[utoipa::path(
    get,
    path = "",
    params(BankAccountTransactionQueryParams),
    responses(
        (status = 200, description = "Successfully got transactions", body = BankAccountHistory),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No bank account or transactions found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_bank_account_transactions(
    State(state): State<ToiState>,
    Query(params): Query<BankAccountTransactionQueryParams>,
) -> Result<Json<BankAccountHistory>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let (bank_account, transaction_ids) =
        search_bank_account_transactions(&state, params, &mut conn).await?;
    let transactions = schema::transactions::table
        .select(Transaction::as_select())
        .filter(schema::transactions::id.eq_any(transaction_ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    let bank_account_history = BankAccountHistory {
        bank_account,
        transactions,
    };
    Ok(Json(bank_account_history))
}

/// Get transactions.
///
/// Example queries for getting transactions using this endpoint:
/// - Get all transactions where
/// - List all transactions
/// - What transactions do I have
/// - How many transactions do I have
#[utoipa::path(
    get,
    path = "",
    params(TransactionQueryParams),
    responses(
        (status = 200, description = "Successfully got transactions", body = [LinkedTransaction]),
        (status = 400, description = "Default JSON elements configured by the user are invalid"),
        (status = 404, description = "No transactions found"),
        (status = 422, description = "Error when parsing a response from a model API"),
        (status = 502, description = "Error when forwarding request to model APIs")
    )
)]
#[axum::debug_handler]
pub async fn get_matching_transactions(
    State(state): State<ToiState>,
    Query(params): Query<TransactionQueryParams>,
) -> Result<Json<Vec<LinkedTransaction>>, (StatusCode, String)> {
    let mut conn = state.pool.get().await.map_err(utils::internal_error)?;
    let transaction_ids = search_transactions(&state, params, &mut conn).await?;
    let linked_transactions = schema::transactions::table
        .select(LinkedTransaction::as_select())
        .filter(schema::transactions::id.eq_any(transaction_ids))
        .load(&mut conn)
        .await
        .map_err(utils::diesel_error)?;
    Ok(Json(linked_transactions))
}
