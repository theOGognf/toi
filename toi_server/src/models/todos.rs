use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Queryable, Selectable, prelude::Insertable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{models::search::SimilaritySearchParams, utils};

#[derive(Deserialize, Queryable, Selectable, Serialize, ToSchema)]
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

#[derive(Deserialize, JsonSchema, ToSchema)]
pub struct NewTodoRequest {
    /// Todo item to add.
    pub item: String,
    /// Optional datetime the todo is due in ISO format.
    pub due_at: Option<DateTime<Utc>>,
    /// Optional datetime the todo was completed in ISO format.
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Builder, Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct CompleteTodoRequest {
    /// Optional datetime the todo was completed in ISO format.
    ///
    /// Defaults to current datetime.
    #[serde(default)]
    pub completed_at: DateTime<Utc>,
    /// Parameters for performing similarity search against todos.
    pub similarity_search_params: Option<SimilaritySearchParams>,
    /// Filter on todos created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on todos created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// Filter on todos due after this ISO formatted datetime.
    pub due_from: Option<DateTime<Utc>>,
    /// Filter on todos due before this ISO formatted datetime.
    pub due_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved todos.
    pub order_by: Option<utils::OrderBy>,
    /// Max number of todos to return from the search.
    pub limit: Option<i64>,
}

#[derive(Builder, Deserialize, Serialize, JsonSchema, IntoParams)]
pub struct TodoQueryParams {
    /// Parameters for performing similarity search against todos.
    pub similarity_search_params: Option<SimilaritySearchParams>,
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
    /// How to order results for retrieved todos.
    pub order_by: Option<utils::OrderBy>,
    /// Max number of todos to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}
