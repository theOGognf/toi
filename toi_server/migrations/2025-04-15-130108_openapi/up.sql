-- Your SQL goes here
CREATE TABLE IF NOT EXISTS openapi (
    id SERIAL PRIMARY KEY,
    spec JSONB NOT NULL,
    embedding VECTOR NOT NULL
);
