use bon::Builder;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use pgvector::Vector;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{models::search::SimilaritySearchParams, utils};

#[derive(Deserialize, Queryable, Selectable, Serialize, ToSchema)]
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

impl From<(Contact, Vector)> for NewContact {
    fn from(value: (Contact, Vector)) -> Self {
        let (contact, embedding) = value;
        Self {
            first_name: contact.first_name,
            last_name: contact.last_name,
            email: contact.email,
            phone: contact.phone,
            birthday: contact.birthday,
            relationship: contact.relationship,
            embedding,
        }
    }
}

impl From<(NewContactRequest, Vector)> for NewContact {
    fn from(value: (NewContactRequest, Vector)) -> Self {
        let (new_contact_request, embedding) = value;
        Self {
            first_name: new_contact_request.first_name,
            last_name: new_contact_request.last_name,
            email: new_contact_request.email,
            phone: new_contact_request.phone,
            birthday: new_contact_request.birthday,
            relationship: new_contact_request.relationship,
            embedding,
        }
    }
}

#[derive(Deserialize, Serialize, JsonSchema, ToSchema)]
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

#[derive(Builder, Deserialize, Serialize, JsonSchema, IntoParams)]
pub struct ContactQueryParams {
    /// Parameters for performing similarity search against contacts.
    #[serde(flatten)]
    pub similarity_search_params: Option<SimilaritySearchParams>,
    /// Filter on contacts created after this ISO formatted datetime.
    pub created_from: Option<DateTime<Utc>>,
    /// Filter on contacts created before this ISO formatted datetime.
    pub created_to: Option<DateTime<Utc>>,
    /// How to order results for retrieved contacts.
    pub order_by: Option<utils::OrderBy>,
    /// Max number of contacts to return from the search.
    #[param(minimum = 1)]
    pub limit: Option<i64>,
}

impl From<UpdateContactRequest> for ContactQueryParams {
    fn from(value: UpdateContactRequest) -> Self {
        Self {
            similarity_search_params: value.similarity_search_params,
            created_from: value.created_from,
            created_to: value.created_to,
            order_by: value.order_by,
            limit: value.limit,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, JsonSchema, ToSchema)]
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

#[derive(Builder, Clone, Deserialize, Serialize, JsonSchema, ToSchema)]
pub struct UpdateContactRequest {
    /// Things to update about a contact.
    pub contact_updates: ContactUpdates,
    #[serde(flatten)]
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
