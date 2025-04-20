-- Add up migration script here
CREATE TABLE IF NOT EXISTS vacancies (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  available    BOOLEAN NOT NULL,      -- 对应 VacancyArray 中 Available/Unavailable
  submitted_at INTEGER NOT NULL       -- 问卷提交时间
);