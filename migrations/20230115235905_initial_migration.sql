-- Add migration script here
CREATE TABLE quotes (
    id INTEGER PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    quote_date TEXT NOT NULL,
    quote TEXT NOT NULL
)
