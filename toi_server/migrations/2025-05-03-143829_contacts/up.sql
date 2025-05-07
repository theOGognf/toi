-- Your SQL goes here
CREATE TABLE IF NOT EXISTS contacts (
    id INT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    first_name TEXT NOT NULL,
    last_name TEXT,
    email TEXT UNIQUE,
    phone TEXT UNIQUE,
    birthday DATE,
    relationship TEXT,
    embedding VECTOR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
