use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{models::contacts::Contact, models::events::Event, utils};

#[derive(Insertable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::event_participants)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Participant {
    pub event_id: i32,
    pub contact_id: i32,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct Participants {
    /// Unique contact ID.
    pub event: Event,
    /// Contact's first name.
    pub contacts: Vec<Contact>,
}

#[derive(Builder, Deserialize, IntoParams, JsonSchema, Serialize, ToSchema)]
pub struct ParticipantQueryParams {
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    pub event_query: String,
    /// Measure of distance between the query and string it's being
    /// compared to. Only return items whose distance is less than
    /// or equal this value. A lower number restricts the search to
    /// more similar items, while a higher number allows for more
    /// dissimilar items. This defaults to the server-configured
    /// value if left null, which is usually fine for most scenarios.
    #[schema(minimum = 0.01, maximum = 0.95)]
    pub event_distance_threshold: Option<f64>,
    /// Measure of similarity between the query and string it's being
    /// compared to. Only return items whose distance is greater than
    /// or equal this value. A higher number restricts the search to
    /// more similar items, while a lower number allows for more
    /// dissimilar items. This defaults to the server-configured
    /// value if left null, which is usually fine for most scenarios.
    #[schema(minimum = 0.01, maximum = 0.95)]
    pub event_similarity_threshold: Option<f64>,
    /// Filter on events created after this ISO formatted datetime.
    pub event_created_from: Option<DateTime<Utc>>,
    /// Filter on events created before this ISO formatted datetime.
    pub event_created_to: Option<DateTime<Utc>>,
    /// Filter on events starting after this ISO formatted datetime.
    pub event_starts_from: Option<DateTime<Utc>>,
    /// Filter on events starting before this ISO formatted datetime.
    pub event_starts_to: Option<DateTime<Utc>>,
    /// Filter on events ending after this ISO formatted datetime.
    pub event_ends_from: Option<DateTime<Utc>>,
    /// Filter on events ending before this ISO formatted datetime.
    pub event_ends_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved events.
    pub event_order_by: Option<utils::OrderBy>,
    /// User query string to compare embeddings against. Basically,
    /// if the user is asking something like "what color is my jacket?",
    /// then the query string should be something like "jacket color" or
    /// the user's original question.
    /// This can be left empty or null to ignore similarity search
    /// in cases where the user wants to filter by other params
    /// (e.g., get items by date or get all items).
    pub contact_query: String,
    /// Measure of distance between the query and string it's being
    /// compared to. Only return items whose distance is less than
    /// or equal this value. A lower number restricts the search to
    /// more similar items, while a higher number allows for more
    /// dissimilar items. This defaults to the server-configured
    /// value if left null, which is usually fine for most scenarios.
    #[schema(minimum = 0.01, maximum = 0.95)]
    pub contact_distance_threshold: Option<f64>,
    /// Measure of similarity between the query and string it's being
    /// compared to. Only return items whose distance is greater than
    /// or equal this value. A higher number restricts the search to
    /// more similar items, while a lower number allows for more
    /// dissimilar items. This defaults to the server-configured
    /// value if left null, which is usually fine for most scenarios.
    #[schema(minimum = 0.01, maximum = 0.95)]
    pub contact_similarity_threshold: Option<f64>,
    /// Max number of contacts to return from the search.
    #[param(minimum = 1)]
    pub contact_limit: Option<i64>,
}
