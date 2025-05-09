-- Add up migration script here
CREATE TABLE IF NOT EXISTS availables (
    user_id INTEGER NOT NULL,
    stamp INTEGER NOT NULL
); 