-- Your SQL goes here
CREATE TABLE IF NOT EXISTS todos (
    id SERIAL PRIMARY KEY,
    item TEXT NOT NULL,
    embedding VECTOR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    due_at TIMESTAMPTZ
);
