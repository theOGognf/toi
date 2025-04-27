-- Your SQL goes here
CREATE TABLE IF NOT EXISTS openapi (
    id SERIAL PRIMARY KEY,
    path TEXT NOT NULL,
    method TEXT NOT NULL,
    spec JSONB NOT NULL,
    embedding VECTOR NOT NULL
);
