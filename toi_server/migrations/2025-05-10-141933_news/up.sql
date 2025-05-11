-- Your SQL goes here
CREATE TABLE IF NOT EXISTS news (
    alias TEXT PRIMARY KEY,
    url TEXT,
    updated_at TIMESTAMPTZ DEFAULT now()
);
