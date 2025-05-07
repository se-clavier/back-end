INSERT INTO rooms (id, name) VALUES (1, 'room1');

INSERT INTO spares (id, room_id, stamp, begin_at, end_at, week, assignee) 
    VALUES 
    (1, 1, 0, "begin1", "end1", "week1", NULL),
    (2, 1, 0, "begin2", "end2", "week2", 1),
    (3, 1, 0, "begin", "end", "schedule", NULL);