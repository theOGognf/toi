-- Your SQL goes here
CREATE TABLE IF NOT EXISTS places (
    id INT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    address TEXT UNIQUE,
    phone TEXT UNIQUE,
    embedding VECTOR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
