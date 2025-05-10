INSERT INTO rooms (id, name) VALUES (1, 'room1');

INSERT INTO spares (id, room_id, stamp, begin_at, end_at, week, assignee, status) 
    VALUES 
    (1, 1, 0, "P0Y0M0DT8H0M0S", "P0Y0M0DT10H0M0S", "2000-W18", NULL, "None"),
    (2, 1, 0, "P0Y0M0DT8H0M0S", "P0Y0M0DT10H0M0S", "2000-W19", 1, "None"),
    (3, 1, 0, "P0Y0M0DT8H0M0S", "P0Y0M0DT10H0M0S", "schedule", NULL, "None"),
    (4, 1, 0, "P0Y0M0DT8H0M0S", "P0Y0M0DT10H0M0S", "2000-W20", 1, "CheckedIn");