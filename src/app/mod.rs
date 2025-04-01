mod hash;
mod user;

use api::{APICollection, API};
use axum::{extract::State, response::Response, routing::post, Json, Router};
use hash::Hasher;
use serde::Serialize;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use user::UserAPI;

#[derive(Debug, Clone)]
/// Application state
struct AppState {
    /// SQLite connection pool
    /// This pool is used to access the SQLite database
    database_pool: SqlitePool,
    password_hasher: Hasher,
}

/// Handler for the root path
async fn handler(
    State(app): State<AppState>,
    Json(body): Json<APICollection>,
) -> Result<Json<impl Serialize>, Response> {
    Ok(Json(app.handle(body).await))
}

/// Create a new Axum router with the given pool
pub fn app(pool: SqlitePool, salt: &str) -> Router {
    Router::new()
        .route("/", post(handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(AppState {
            database_pool: pool,
            password_hasher: Hasher::new(salt),
        })
}

/// Create a new SQLite connection pool
pub async fn connect_pool(url: &str) -> SqlitePool {
    // Create a new SQLite database if it doesn't exist
    if !Sqlite::database_exists(url).await.unwrap() {
        Sqlite::create_database(url).await.unwrap();
    }

    // Create a connection pool to the SQLite database
    let pool = SqlitePool::connect(url).await.unwrap();
    tracing::info!("Connected to SQLite database {}", url);

    // Run migrations
    sqlx::migrate!().run(&pool).await.unwrap();

    pool
}

impl API for AppState {
    async fn login(&self, req: api::LoginRequest) -> api::LoginResponse {
        UserAPI::login(self, req).await
    }
    async fn register(&self, req: api::RegisterRequest) -> api::RegisterResponse {
        UserAPI::register(self, req).await
    }
    async fn get_user(&self, req: api::Id) -> api::User {
        UserAPI::get_user(self, req).await
    }
}

#[cfg(test)]
mod test {
    use tracing_subscriber::util::SubscriberInitExt;

    use super::*;

    #[tokio::test]
    async fn test_connect_pool() {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        connect_pool("sqlite::memory:").await;
    }
}
