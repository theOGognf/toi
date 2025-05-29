use bon::Builder;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::utils;

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Event {
    /// Unique event ID.
    pub id: i32,
    /// Event description.
    pub description: String,
    /// Datetime the event was created in ISO format.
    pub created_at: DateTime<Utc>,
    /// Datetime the event starts in ISO format.
    pub starts_at: DateTime<Utc>,
    /// Datetime the event ends in ISO format.
    pub ends_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewEvent {
    pub description: String,
    pub embedding: Vector,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct NewEventRequest {
    /// Event description to add.
    pub description: String,
    /// Datetime the event starts in ISO format.
    pub starts_at: DateTime<Utc>,
    /// Datetime the event ends in ISO format.
    pub ends_at: DateTime<Utc>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct EventSearchParams {
    /// Select events using their database-generated IDs rather than searching
    /// for them.
    pub ids: Option<Vec<i32>>,
    /// Event day search parameter. What kind of search depends on
    /// the `falls_on` field.
    pub event_day: Option<NaiveDate>,
    /// What kind of calendar object the event falls on. Used
    /// to search if an event falls on the month of, week of,
    /// or day of `event_day`.
    pub event_day_falls_on: Option<utils::DateFallsOn>,
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
    /// Filter on events created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on events created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved events.
    pub order_by: Option<utils::OrderBy>,
    /// Limit the max number of events to return from the search.
    pub limit: Option<i64>,
}
