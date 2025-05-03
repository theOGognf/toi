// @generated automatically by Diesel CLI.

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
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    event_participants (event_id, contact_id) {
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

    openapi (path, method) {
        path -> Text,
        method -> Text,
        description -> Text,
        params -> Nullable<Jsonb>,
        body -> Nullable<Jsonb>,
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

diesel::joinable!(event_participants -> contacts (contact_id));
diesel::joinable!(event_participants -> events (event_id));

diesel::allow_tables_to_appear_in_same_query!(
    contacts,
    event_participants,
    events,
    notes,
    openapi,
    todos,
);
