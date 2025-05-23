use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    models::contacts::Contact,
    models::events::{Event, EventFallsOnSearchParams},
    utils,
};

#[derive(Insertable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::event_attendees)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Attendee {
    pub event_id: i32,
    pub contact_id: i32,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct Attendees {
    /// Matching event.
    pub event: Event,
    /// Matching contacts.
    pub contacts: Vec<Contact>,
}

#[derive(Builder, Deserialize, IntoParams, JsonSchema, Serialize, ToSchema)]
pub struct AttendeeQueryParams {
    /// Select an event using its database-generated IDs rather than
    /// searching for it first.
    pub event_id: Option<i32>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    pub event_query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `True` is useful for cases where the user is looking to match
    /// to a specific phrase, name, or words.
    #[serde(default)]
    pub event_use_reranking_filter: bool,
    /// Filter on events created after this ISO formatted datetime.
    pub event_created_from: Option<DateTime<Utc>>,
    /// Filter on events created before this ISO formatted datetime.
    pub event_created_to: Option<DateTime<Utc>>,
    /// Parameters for performing a search against event days.
    /// This can be left empty or null to ignore these search options
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    #[serde(flatten)]
    pub event_day_falls_on_search_params: Option<EventFallsOnSearchParams>,
    /// How to order results for retrieved events.
    pub event_order_by: Option<utils::OrderBy>,
    /// Search contacts using their database-generated IDs rather than
    /// searching for them first.
    pub contact_ids: Option<Vec<i32>>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    pub contact_query: Option<String>,
    /// Whether to match the query string more closely using a reranking -based
    /// approach. `True` is useful for cases where the user is looking to match
    /// to a specific phrase, name, or words.
    #[serde(default)]
    pub contact_use_reranking_filter: bool,
    /// Limit the max number of contacts to return from the search.
    #[param(minimum = 1)]
    pub contact_limit: Option<i64>,
}
