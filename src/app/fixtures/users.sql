-- This file is used to insert test data into the table users and user_roles.

insert into users (username, password) values ("testuser", "password123");
insert into user_roles (user_id, role_type) values (1, "user");
