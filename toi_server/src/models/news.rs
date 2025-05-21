use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::{IntoParams, ToSchema};

#[derive(Insertable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::news)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct News {
    pub alias: String,
    pub tinyurl: String,
    pub title: Option<String>,
    pub url: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::news)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Alias {
    pub alias: String,
    pub tinyurl: String,
}

#[derive(Debug, Deserialize, PartialEq, Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = crate::schema::news)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRedirect {
    pub tinyurl: String,
    pub title: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::news)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAlias {
    pub alias: String,
    pub tinyurl: String,
    pub updated_at: Option<DateTime<Utc>>,
}

impl NewAlias {
    pub fn new(bind_addr: &str, alias: String) -> Self {
        let tinyurl = format!("http://{bind_addr}/news/{alias}");
        Self {
            alias,
            tinyurl,
            updated_at: None,
        }
    }
}

#[derive(AsChangeset, Default)]
#[diesel(table_name = crate::schema::news)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ExpiredRedirect {
    pub title: Option<String>,
    pub url: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Builder, Deserialize, IntoParams, JsonSchema, Serialize, ToSchema)]
pub struct GetNewsRequest {
    /// Limit the search to titles/descriptions matching this query.
    /// E.g., if the user says "get news for apnews.com", then this
    /// should be "apnews.com". This should be null if the user isn't
    /// requesting something specific.
    pub query: Option<String>,
    /// Limit the search to articles published up to this many hours in the past.
    #[param(minimum = 1, maximum = 24)]
    #[schemars(range(min = 1, max = 24))]
    pub when: Option<u8>,
}

impl From<GetNewsRequest> for (&'static str, Value) {
    fn from(value: GetNewsRequest) -> Self {
        let mut s = vec![];
        if let Some(search) = value.query {
            s.push(search);
        }
        if let Some(when) = value.when {
            s.push(format!("when:{when}h"));
        }
        if s.is_empty() {
            ("https://news.google.com/rss", json!({}))
        } else {
            (
                "https://news.google.com/rss/search",
                json!(
                    {
                        "q": s.join("+")
                    }
                ),
            )
        }
    }
}
