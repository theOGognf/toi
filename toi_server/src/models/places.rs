use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

use crate::utils;

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::places)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Place {
    /// Unique place ID.
    pub id: i32,
    /// Place's name.
    pub name: String,
    /// Place's description.
    pub description: String,
    /// Place's address.
    pub address: Option<String>,
    /// Place's phone number in XXX-XXX-XXXX format.
    pub phone: Option<String>,
    /// Datetime the place was created in ISO format.
    pub created_at: DateTime<Utc>,
}

impl Place {
    pub fn update(&mut self, updates: PlaceUpdates) {
        if let Some(name) = updates.name {
            self.name = name;
        }
        if let Some(description) = updates.description {
            self.description = description;
        }
        if let Some(address) = updates.address {
            self.address = Some(address);
        }
        if let Some(phone) = updates.phone {
            self.phone = Some(phone);
        }
    }
}

#[derive(AsChangeset, Insertable)]
#[diesel(table_name = crate::schema::places)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewPlace {
    pub name: String,
    pub description: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub embedding: Vector,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct NewPlaceRequest {
    /// Place's name.
    pub name: String,
    /// Place's description.
    pub description: String,
    /// Place's address.
    pub address: Option<String>,
    /// Place's phone number in XXX-XXX-XXXX format.
    pub phone: Option<String>,
}

impl fmt::Display for NewPlaceRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items: Vec<String> = [
            ("Name", Some(&self.name)),
            ("Description", Some(&self.description)),
            ("Address", self.address.as_ref()),
            ("Phone", self.phone.as_ref()),
        ]
        .iter()
        .filter_map(|(k, opt)| opt.map(|v| format!("{k}: {v}")))
        .collect();
        let repr = items.join("\n");
        write!(f, "{repr}")
    }
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct PlaceSearchParams {
    /// Select places according to their database-generated IDs rather
    /// than searching for them.
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
    /// Filter on places created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on places created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved places.
    pub order_by: Option<utils::OrderBy>,
    /// Limit the max number of places to return from the search.
    pub limit: Option<i64>,
}

#[derive(Builder, Clone, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct PlaceUpdates {
    /// Place's name.
    pub name: Option<String>,
    /// Place's description.
    pub description: Option<String>,
    /// Place's address.
    pub address: Option<String>,
    /// Place's phone number in XXX-XXX-XXXX format.
    pub phone: Option<String>,
}

#[derive(Builder, Clone, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct UpdatePlaceRequest {
    /// Update a place using their database-generated ID rather than
    /// searching for them.
    pub id: Option<i32>,
    /// Things to update about a place.
    pub place_updates: PlaceUpdates,
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
    /// Filter on places created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on places created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved places.
    pub order_by: Option<utils::OrderBy>,
}
