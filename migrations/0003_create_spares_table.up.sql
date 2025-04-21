-- Add up migration script here
CREATE TABLE IF NOT EXISTS spares (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  room_id     INTEGER    NOT NULL
                    REFERENCES rooms(id) ON DELETE CASCADE,
  assignee    INTEGER,                -- 借用人 ID，NULL 表示未借出
  stamp       INTEGER,                -- 一周里面的 INDEX
  begin_at    TEXT       NOT NULL,    -- 开始时间
  end_at      TEXT       NOT NULL,    -- 结束时间
  week        TEXT       NOT NULL     -- 本条记录所属周编号
);