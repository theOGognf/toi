-- Your SQL goes here
CREATE TABLE IF NOT EXISTS news (
    tinyurl TEXT PRIMARY KEY,
    url TEXT,
    description TEXT,
    embedding VECTOR,
    created_at TIMESTAMPTZ DEFAULT now()
);
