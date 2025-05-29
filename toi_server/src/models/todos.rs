use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::utils;

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::todos)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Todo {
    /// Unique todo ID.
    pub id: i32,
    /// Todo item.
    pub item: String,
    /// Datetime the todo was created in ISO format.
    pub created_at: DateTime<Utc>,
    /// Datetime the todo is due in ISO format.
    pub due_at: Option<DateTime<Utc>>,
    /// Datetime the todo was completed in ISO format.
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::todos)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTodo {
    pub item: String,
    pub embedding: Vector,
    pub due_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct NewTodoRequest {
    /// Todo item to add.
    pub item: String,
    /// Optional datetime the todo is due in ISO format.
    pub due_at: Option<DateTime<Utc>>,
    /// Optional datetime the todo was completed in ISO format.
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct CompleteTodoRequest {
    /// Complete todos using their database-generated IDs rather than
    /// searching for them first.
    pub ids: Option<Vec<i32>>,
    /// Optional datetime the todo was completed in ISO format.
    ///
    /// Defaults to current datetime.
    #[serde(default)]
    pub completed_at: DateTime<Utc>,
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
    /// Filter on todos created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on todos created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// Filter on todos due after this ISO formatted datetime.
    pub due_from: Option<DateTime<Utc>>,
    /// Filter on todos due before this ISO formatted datetime.
    pub due_to: Option<DateTime<Utc>>,
    /// Whether to include or exclude todos that are incomplete.
    pub incomplete: Option<utils::Scope>,
    /// Whether to include or exclude todos that are never due.
    pub never_due: Option<utils::Scope>,
    /// How to order results for retrieved todos.
    pub order_by: Option<utils::OrderBy>,
    /// Limit the max number of todos to return from the search.
    pub limit: Option<i64>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct TodoSearchParams {
    /// Select todos using their database-generated IDs rather than
    /// searching for them.
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
    /// Filter on todos created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on todos created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// Filter on todos due after this ISO formatted datetime.
    pub due_from: Option<DateTime<Utc>>,
    /// Filter on todos due before this ISO formatted datetime.
    pub due_to: Option<DateTime<Utc>>,
    /// Filter on todos completed after this ISO formatted datetime.
    pub completed_from: Option<DateTime<Utc>>,
    /// Filter on todos completed before this ISO formatted datetime.
    pub completed_to: Option<DateTime<Utc>>,
    /// Whether to include or exclude todos that are incomplete.
    pub incomplete: Option<utils::Scope>,
    /// Whether to include or exclude todos that are never due.
    pub never_due: Option<utils::Scope>,
    /// How to order results for retrieved todos.
    pub order_by: Option<utils::OrderBy>,
    /// Limit the max number of todos to return from the search.
    pub limit: Option<i64>,
}
