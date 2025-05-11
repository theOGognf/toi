-- Your SQL goes here
CREATE TABLE IF NOT EXISTS news (
    alias TEXT PRIMARY KEY,
    tinyurl TEXT NOT NULL,
    title TEXT,
    url TEXT,
    updated_at TIMESTAMPTZ
);
