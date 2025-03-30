-- Add up migration script here
create table if not exists users (
    id integer primary key autoincrement,
    username text not null,
    password text not null
);
create table if not exists user_roles (
    user_id integer not null,
    role_name text not null,
    foreign key (user_id) references users(id)
);