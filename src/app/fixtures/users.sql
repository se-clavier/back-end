-- This file is used to insert test data into the table users and user_roles.

insert into users (username, password) values ("testuser", "$argon2id$v=19$m=19456,t=2,p=1$YmFzZXNhbHQ$3i0y+gypIZM5SX0/X2fkMRZI8fXiw6PP/5/JLHj+Tpg"); -- password: password123
insert into user_roles (user_id, role_type) values (1, "user");
