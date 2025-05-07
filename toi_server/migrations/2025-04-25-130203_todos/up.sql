-- Your SQL goes here
CREATE TABLE IF NOT EXISTS todos (
    id INT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    item TEXT NOT NULL,
    embedding VECTOR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    due_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);
