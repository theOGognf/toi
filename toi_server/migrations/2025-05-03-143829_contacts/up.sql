-- Your SQL goes here
CREATE TABLE IF NOT EXISTS contacts (
    id SERIAL PRIMARY KEY,
    first_name TEXT NOT NULL,
    last_name TEXT,
    email TEXT UNIQUE,
    phone TEXT UNIQUE,
    birthday DATE,
    relationship TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
