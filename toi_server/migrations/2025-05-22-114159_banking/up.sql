-- Your SQL goes here
CREATE TABLE IF NOT EXISTS bank_accounts (
    id INT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    description TEXT NOT NULL,
    embedding VECTOR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS transactions (
    id INT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    bank_account_id INT NOT NULL REFERENCES bank_accounts(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    amount REAL NOT NULL,
    embedding VECTOR NOT NULL,
    posted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
