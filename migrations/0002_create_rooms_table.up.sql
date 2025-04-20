-- Add up migration script here
CREATE TABLE IF NOT EXISTS rooms (
  id   INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT    NOT NULL UNIQUE
);
