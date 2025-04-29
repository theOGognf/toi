-- Your SQL goes here
CREATE TABLE IF NOT EXISTS openapi (
    path TEXT,
    method TEXT,
    params JSONB,
    body JSONB,
    embedding VECTOR NOT NULL,
    PRIMARY KEY(path, method)
);
