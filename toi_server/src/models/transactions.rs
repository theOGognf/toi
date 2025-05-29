use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{models::accounts::BankAccount, utils};

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LinkedTransaction {
    pub bank_account_id: i32,
    pub id: i32,
    pub description: String,
    pub amount: f32,
    pub posted_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewLinkedTransaction {
    pub bank_account_id: i32,
    pub description: String,
    pub amount: f32,
    pub embedding: Vector,
    pub posted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Transaction {
    pub id: i32,
    pub description: String,
    pub amount: f32,
    pub posted_at: DateTime<Utc>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct TransactionSearchParams {
    /// Select a bank account using its database-generated IDs rather than
    /// searching for it first.
    #[serde(skip)]
    pub bank_account_id: Option<i32>,
    /// Select transactions using their database-generated IDs rather than searching
    /// for them.
    pub ids: Option<Vec<i32>>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question. This can be left empty to ignore
    /// similarity search in cases where the user wants to filter by
    /// other means or get all items.
    pub query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `true` is useful for cases where the user is looking to match
    /// to specific words or phrases, whereas `false` is useful for more broad
    /// matching.
    pub use_reranking_filter: Option<bool>,
    /// Filter on transactions posted after this ISO formatted datetime.
    pub posted_from: Option<DateTime<Utc>>,
    /// Filter on transactions posted before this ISO formatted datetime.
    pub posted_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved transactions.
    pub order_by: Option<utils::OrderBy>,
    /// Limit the max number of transactions to return from the search.
    pub limit: Option<i64>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct NewBankAccountTransactionRequest {
    /// Select a bank account using its database-generated IDs rather than
    /// searching for it first.
    pub bank_account_id: Option<i32>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    pub bank_account_query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `true` is useful for cases where the user is looking to match
    /// to a specific phrase, name, or words.
    pub bank_account_use_reranking_filter: Option<bool>,
    /// Filter on bank accounts created after this ISO formatted datetime.
    pub bank_account_created_from: Option<DateTime<Utc>>,
    /// Filter on bank accounts created before this ISO formatted datetime.
    pub bank_account_created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved bank accounts.
    pub bank_account_order_by: Option<utils::OrderBy>,
    /// New transaction description.
    pub transaction_description: String,
    /// Transaction amount.
    pub transaction_amount: f32,
    /// Time the transaction was made/posted in ISO format.
    pub transaction_posted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct BankAccountHistory {
    /// Matching bank account.
    pub bank_account: BankAccount,
    /// Matching transactions.
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct BankAccountTransaction {
    /// Matching bank account.
    pub bank_account: BankAccount,
    /// New transaction.
    pub transaction: Transaction,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct BankAccountTransactionSearchParams {
    /// Select a bank account using its database-generated IDs rather than
    /// searching for it first.
    pub bank_account_id: Option<i32>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    pub bank_account_query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `true` is useful for cases where the user is looking to match
    /// to a specific phrase, name, or words.
    pub bank_account_use_reranking_filter: Option<bool>,
    /// Filter on bank accounts created after this ISO formatted datetime.
    pub bank_account_created_from: Option<DateTime<Utc>>,
    /// Filter on bank accounts created before this ISO formatted datetime.
    pub bank_account_created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved bank accounts.
    pub bank_account_order_by: Option<utils::OrderBy>,
    /// Search transactions using their database-generated IDs rather than
    /// searching for them first.
    pub transaction_ids: Option<Vec<i32>>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    pub transaction_query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `true` is useful for cases where the user is looking to match
    /// to a specific phrase, name, or words.
    pub transaction_use_reranking_filter: Option<bool>,
    /// Filter on transactions created after this ISO formatted datetime.
    pub transaction_posted_from: Option<DateTime<Utc>>,
    /// Filter on transactions created before this ISO formatted datetime.
    pub transaction_posted_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved transactions.
    pub transaction_order_by: Option<utils::OrderBy>,
    /// Limit the max number of transactions to return from the search.
    pub transaction_limit: Option<i64>,
}
