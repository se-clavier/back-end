-- Add up migration script here
CREATE TABLE IF NOT EXISTS spares (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  room_id     INTEGER    NOT NULL
                    REFERENCES rooms(id) ON DELETE CASCADE,
  assignee    INTEGER,                -- 借用人 ID，NULL 表示未借出
  stamp       INTEGER,                -- 一周里面的 INDEX
  taken_at    TEXT       NOT NULL,    -- 借出时间（UNIX 时间戳)
  returned_at TEXT,                  -- 归还时间，改成字符串，为 NULL 表示尚未归还
  week        TEXT       NOT NULL     -- 本条记录所属周编号
);