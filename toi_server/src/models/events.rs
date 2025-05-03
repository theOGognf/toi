use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{models::search::SimilaritySearchParams, utils};

#[derive(Deserialize, Queryable, Selectable, Serialize, ToSchema)]
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

#[derive(Builder, Deserialize, Serialize, JsonSchema, IntoParams)]
pub struct EventQueryParams {
    /// Parameters for performing similarity search against events.
    #[serde(flatten)]
    pub similarity_search_params: Option<SimilaritySearchParams>,
    /// Filter on events created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on events created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// Filter on events starting after this ISO formatted datetime.
    pub starts_from: Option<DateTime<Utc>>,
    /// Filter on events starting before this ISO formatted datetime.
    pub starts_to: Option<DateTime<Utc>>,
    /// Filter on events ending after this ISO formatted datetime.
    pub ends_from: Option<DateTime<Utc>>,
    /// Filter on events ending before this ISO formatted datetime.
    pub ends_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved events.
    pub order_by: Option<utils::OrderBy>,
    /// Max number of events to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}
