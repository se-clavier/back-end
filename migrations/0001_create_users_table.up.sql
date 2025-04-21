-- Add up migration script here
create table if not exists users (
    id integer primary key autoincrement,
    username text not null unique,
    password text not null
);
create table if not exists user_roles (
    user_id integer not null,
    role_type text not null,
    foreign key (user_id) references users(id)
);