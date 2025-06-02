// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    bank_accounts (id) {
        id -> Int4,
        description -> Text,
        embedding -> Vector,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    contacts (id) {
        id -> Int4,
        first_name -> Text,
        last_name -> Nullable<Text>,
        email -> Nullable<Text>,
        phone -> Nullable<Text>,
        birthday -> Nullable<Date>,
        relationship -> Nullable<Text>,
        embedding -> Vector,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    event_attendees (event_id, contact_id) {
        event_id -> Int4,
        contact_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    events (id) {
        id -> Int4,
        description -> Text,
        embedding -> Vector,
        created_at -> Timestamptz,
        starts_at -> Timestamptz,
        ends_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    news (alias) {
        alias -> Text,
        tinyurl -> Text,
        title -> Nullable<Text>,
        url -> Nullable<Text>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    notes (id) {
        id -> Int4,
        content -> Text,
        embedding -> Vector,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    openapi (id) {
        id -> Int4,
        path -> Text,
        method -> Text,
        description -> Text,
        params -> Nullable<Jsonb>,
        body -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    places (id) {
        id -> Int4,
        name -> Text,
        description -> Text,
        address -> Nullable<Text>,
        phone -> Nullable<Text>,
        embedding -> Vector,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    recipe_tags (recipe_id, tag_id) {
        recipe_id -> Int4,
        tag_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    recipes (id) {
        id -> Int4,
        description -> Text,
        ingredients -> Text,
        instructions -> Text,
        embedding -> Vector,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    searchable_openapi (id) {
        id -> Int4,
        parent_id -> Int4,
        description -> Text,
        embedding -> Vector,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    tags (id) {
        id -> Int4,
        name -> Text,
        embedding -> Vector,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    todos (id) {
        id -> Int4,
        item -> Text,
        embedding -> Vector,
        created_at -> Timestamptz,
        due_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    transactions (id) {
        id -> Int4,
        bank_account_id -> Int4,
        description -> Text,
        amount -> Float4,
        embedding -> Vector,
        posted_at -> Timestamptz,
    }
}

diesel::joinable!(event_attendees -> contacts (contact_id));
diesel::joinable!(event_attendees -> events (event_id));
diesel::joinable!(recipe_tags -> recipes (recipe_id));
diesel::joinable!(recipe_tags -> tags (tag_id));
diesel::joinable!(searchable_openapi -> openapi (parent_id));
diesel::joinable!(transactions -> bank_accounts (bank_account_id));

diesel::allow_tables_to_appear_in_same_query!(
    bank_accounts,
    contacts,
    event_attendees,
    events,
    news,
    notes,
    openapi,
    places,
    recipe_tags,
    recipes,
    searchable_openapi,
    tags,
    todos,
    transactions,
);
