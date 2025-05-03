-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS openapi (
    path TEXT,
    method TEXT,
    description TEXT NOT NULL,
    params JSONB,
    body JSONB,
    embedding VECTOR NOT NULL,
    PRIMARY KEY(path, method)
);
