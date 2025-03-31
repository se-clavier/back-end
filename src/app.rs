use crate::config::Config;
use crate::val::SignatureGenerator;
use api::{APICollection, User, API};
use serde::{Deserialize, Serialize};
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use std::collections::HashMap;

#[derive(Clone)]
pub struct App {
    db: HashMap<String, String>,
    pub secret: String,
    pub pool: SqlitePool,
    pub signature_generator: SignatureGenerator,
}

impl App {
    pub fn new(config: Config, pool: SqlitePool) -> Self {
        todo!();
    }
}
