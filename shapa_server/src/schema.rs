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
