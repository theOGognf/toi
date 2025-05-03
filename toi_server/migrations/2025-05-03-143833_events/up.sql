-- Your SQL goes here
CREATE TABLE IF NOT EXISTS events (
    id SERIAL PRIMARY KEY,
    description TEXT NOT NULL,
    embedding VECTOR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS event_participants (
    event_id INT REFERENCES events(id) ON DELETE CASCADE,
    contact_id INT REFERENCES contacts(id) ON DELETE CASCADE,
    PRIMARY KEY (event_id, contact_id)
);
