use bon::Builder;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

use crate::utils;

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::contacts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Contact {
    /// Unique contact ID.
    pub id: i32,
    /// Contact's first name.
    pub first_name: String,
    /// Contact's last name.
    pub last_name: Option<String>,
    /// Contact's email.
    pub email: Option<String>,
    /// Contact's phone number in XXX-XXX-XXXX format.
    pub phone: Option<String>,
    /// Contact's birthday.
    pub birthday: Option<NaiveDate>,
    /// Short description of relationship to the contact.
    pub relationship: Option<String>,
    /// Datetime the contact was created in ISO format.
    pub created_at: DateTime<Utc>,
}

impl Contact {
    pub fn update(&mut self, updates: ContactUpdates) {
        if let Some(first_name) = updates.first_name {
            self.first_name = first_name;
        }
        if let Some(last_name) = updates.last_name {
            self.last_name = Some(last_name);
        }
        if let Some(email) = updates.email {
            self.email = Some(email);
        }
        if let Some(phone) = updates.phone {
            self.phone = Some(phone);
        }
        if let Some(birthday) = updates.birthday {
            self.birthday = Some(birthday);
        }
        if let Some(relationship) = updates.relationship {
            self.relationship = Some(relationship);
        }
    }
}

#[derive(AsChangeset, Insertable)]
#[diesel(table_name = crate::schema::contacts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewContact {
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub birthday: Option<NaiveDate>,
    pub relationship: Option<String>,
    pub embedding: Vector,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct NewContactRequest {
    /// Contact's first name.
    pub first_name: String,
    /// Contact's last name.
    pub last_name: Option<String>,
    /// Contact's email.
    pub email: Option<String>,
    /// Contact's phone number in XXX-XXX-XXXX format.
    pub phone: Option<String>,
    /// Contact's birthday.
    pub birthday: Option<NaiveDate>,
    /// Short description of relationship to the contact.
    pub relationship: Option<String>,
}

impl fmt::Display for NewContactRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items: Vec<String> = [
            ("First Name", Some(&self.first_name)),
            ("Last Name", self.last_name.as_ref()),
            ("Email", self.email.as_ref()),
            ("Phone", self.phone.as_ref()),
            ("Relationship", self.relationship.as_ref()),
        ]
        .iter()
        .filter_map(|(k, opt)| opt.map(|v| format!("{k}: {v}")))
        .collect();
        let repr = items.join("\n");
        write!(f, "{repr}")
    }
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct ContactDeleteParams {
    /// Select contacts according to their database-generated IDs rather
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
    /// Filter on contacts created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on contacts created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved contacts.
    pub order_by: Option<utils::OrderBy>,
    /// Limit the max number of contacts to return from the search.
    pub limit: Option<i64>,
}

#[derive(Builder, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct ContactSearchParams {
    /// Select contacts according to their database-generated IDs rather
    /// than searching for them.
    pub ids: Option<Vec<i32>>,
    /// Birthday search parameter. What kind of search depends on
    /// the `falls_on` field.
    pub birthday: Option<NaiveDate>,
    /// What kind of calendar object the birthday falls on. Used
    /// to search if a contact's birthday falls on the month of, week of,
    /// or day of `birthday`.
    pub birthday_falls_on: Option<utils::DateFallsOn>,
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
    /// Filter on contacts created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on contacts created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved contacts.
    pub order_by: Option<utils::OrderBy>,
    /// Limit the max number of contacts to return from the search.
    pub limit: Option<i64>,
}

#[derive(Builder, Clone, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct ContactUpdates {
    /// Contact's first name.
    pub first_name: Option<String>,
    /// Contact's last name.
    pub last_name: Option<String>,
    /// Contact's email.
    pub email: Option<String>,
    /// Contact's phone number in XXX-XXX-XXXX format.
    pub phone: Option<String>,
    /// Contact's birthday.
    pub birthday: Option<NaiveDate>,
    /// Short description of relationship to the contact.
    pub relationship: Option<String>,
}

#[derive(Builder, Clone, Deserialize, JsonSchema, Serialize, ToSchema)]
pub struct UpdateContactRequest {
    /// Update a contact using their database-generated ID rather than
    /// searching for them.
    pub id: Option<i32>,
    /// Things to update about a contact.
    pub contact_updates: ContactUpdates,
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
    /// Filter on contacts created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on contacts created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved contacts.
    pub order_by: Option<utils::OrderBy>,
}
