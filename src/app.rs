use api::{APICollection, User, API};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::config::Config;
use sqlx::{
    migrate::MigrateDatabase, Sqlite, SqlitePool,
};

#[derive(Clone)]
pub struct App {
	db: HashMap<String, String>,
	secret: String,
    pub pool: SqlitePool,
}

impl App {
	pub fn new(config: Config, pool: SqlitePool) -> Self {
		todo!();
	}
	pub fn get_pool(&self) -> &SqlitePool {
		&self.pool
	}
	pub fn get_secret(&self) -> &String {
		&self.secret
	}
}