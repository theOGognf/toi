-- Your SQL goes here
CREATE TABLE IF NOT EXISTS openapi (
    id SERIAL PRIMARY KEY,
    description TEXT NOT NULL,
    spec JSONB NOT NULL,
    embedding VECTOR NOT NULL
);
