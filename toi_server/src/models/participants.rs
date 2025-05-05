use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    models::contacts::Contact, models::events::Event, models::search::SimilaritySearchParams, utils,
};

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

#[derive(Builder, Deserialize, Serialize, JsonSchema, IntoParams, ToSchema)]
pub struct ParticipantEventQueryParams {
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
}

#[derive(Builder, Deserialize, Serialize, JsonSchema, IntoParams, ToSchema)]
pub struct ParticipantContactQueryParams {
    /// Parameters for performing similarity search against contacts.
    #[serde(flatten)]
    pub similarity_search_params: Option<SimilaritySearchParams>,
    /// Max number of contacts to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}

#[derive(Builder, Deserialize, Serialize, JsonSchema, IntoParams, ToSchema)]
pub struct ParticipantQueryParams {
    pub event_query_params: ParticipantEventQueryParams,
    pub contact_query_params: ParticipantContactQueryParams,
}
