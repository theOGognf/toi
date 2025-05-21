use bon::Builder;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{models::search::SimilaritySearchParams, utils};

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

#[derive(Deserialize, JsonSchema, ToSchema)]
pub struct NewEventRequest {
    /// Event description to add.
    pub description: String,
    /// Datetime the event starts in ISO format.
    pub starts_at: DateTime<Utc>,
    /// Datetime the event ends in ISO format.
    pub ends_at: DateTime<Utc>,
}

#[derive(Builder, Deserialize, IntoParams, JsonSchema, Serialize, ToSchema)]
pub struct EventFallsOnSearchParams {
    /// Event day search parameter. What kind of search depends on
    /// the `falls_on` field.
    #[serde(default)]
    pub event_day: NaiveDate,
    /// What kind of calendar object the event falls on. Used
    /// to search if an event falls on the month of, week of,
    /// or day of `event_day`.
    pub falls_on: utils::DateFallsOn,
}

#[derive(Builder, Deserialize, IntoParams, JsonSchema, Serialize, ToSchema)]
pub struct EventQueryParams {
    /// Select events using their database-generated IDs rather than searching
    /// for them.
    pub ids: Option<Vec<i32>>,
    /// Parameters for performing a search against event days.
    /// This can be left empty or null to ignore these search options
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    #[serde(flatten)]
    pub event_day_falls_on_search_params: Option<EventFallsOnSearchParams>,
    /// Parameters for performing similarity search against events.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    #[serde(flatten)]
    pub similarity_search_params: Option<SimilaritySearchParams>,
    /// Filter on events created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on events created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved events.
    pub order_by: Option<utils::OrderBy>,
    /// Limit the max number of events to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}
