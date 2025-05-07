-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS openapi (
    id INT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    path TEXT NOT NULL,
    method TEXT NOT NULL,
    description TEXT NOT NULL,
    params JSONB,
    body JSONB
);

CREATE TABLE IF NOT EXISTS searchable_openapi (
    id INT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    parent_id INT NOT NULL REFERENCES openapi(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    embedding VECTOR NOT NULL
);
