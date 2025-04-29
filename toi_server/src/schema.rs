// @generated automatically by Diesel CLI.

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

diesel::allow_tables_to_appear_in_same_query!(notes, openapi, todos,);
