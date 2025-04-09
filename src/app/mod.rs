mod hash;
mod sign;
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
    signer: sign::Signer,
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
            signer: sign::Signer::new(salt),
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
    async fn test_auth_echo(
        &self,
        _req: api::TestAuthEchoRequest,
        _auth: api::Auth,
    ) -> api::TestAuthEchoResponse {
        if !self.signer.verify(&_auth) {
            return api::TestAuthEchoResponse {
                data: "Invalid signature".to_string(),
            };
        }
        api::TestAuthEchoResponse { data: _req.data }
    }

    async fn validate(&self, _role: api::Role, _auth: api::Auth) -> api::Result<api::Auth> {
        self.signer.validate(_role, _auth)
    }
}

#[cfg(test)]
mod test {
    use api::{Auth, Result, Role, TestAuthEchoRequest};
    use tracing_subscriber::util::SubscriberInitExt;

    use super::*;

    #[tokio::test]
    async fn test_connect_pool() {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        connect_pool("sqlite::memory:").await;
    }
    #[tokio::test]
    async fn test_test_auth_echo_valid() {
        let salt = "mysecret";
        let pool = connect_pool("sqlite::memory:").await;
        let app_state = AppState {
            database_pool: pool,
            password_hasher: Hasher::new(salt),
            signer: sign::Signer::new(salt),
        };

        let mut auth = Auth {
            id: 1,
            roles: vec![Role::admin],
            signature: String::new(),
        };
        auth = app_state.signer.sign(auth);

        let req = TestAuthEchoRequest {
            data: "Hello, world!".to_string(),
        };

        let resp = app_state.test_auth_echo(req, auth).await;
        assert_eq!(resp.data, "Hello, world!");
    }

    #[tokio::test]
    async fn test_test_auth_echo_invalid() {
        let salt = "mysecret";
        let pool = connect_pool("sqlite::memory:").await;
        let app_state = AppState {
            database_pool: pool,
            password_hasher: Hasher::new(salt),
            signer: sign::Signer::new(salt),
        };
        let auth = Auth {
            id: 2,
            roles: vec![Role::admin],
            signature: "bad_signature".to_string(),
        };
        let req = TestAuthEchoRequest {
            data: "Test data".to_string(),
        };
        let resp = app_state.test_auth_echo(req, auth).await;
        assert_eq!(resp.data, "Invalid signature");
    }

    #[tokio::test]
    async fn test_validate_authorized() {
        let salt = "mysecret";
        let pool = connect_pool("sqlite::memory:").await;
        let app_state = AppState {
            database_pool: pool,
            password_hasher: Hasher::new(salt),
            signer: sign::Signer::new(salt),
        };
        let mut auth = Auth {
            id: 3,
            roles: vec![Role::admin, Role::user],
            signature: String::new(),
        };
        auth = app_state.signer.sign(auth);
        let result = app_state.validate(Role::admin, auth).await;
        match result {
            Result::Ok(valid_auth) => {
                assert!(valid_auth.roles.contains(&Role::admin));
            }
            Result::Unauthorized => panic!("Expected authorized, but got unauthorized"),
        }
    }

    #[tokio::test]
    async fn test_validate_unauthorized() {
        let salt = "mysecret";
        let pool = connect_pool("sqlite::memory:").await;
        let app_state = AppState {
            database_pool: pool,
            password_hasher: Hasher::new(salt),
            signer: sign::Signer::new(salt),
        };

        let mut auth = Auth {
            id: 4,
            roles: vec![Role::user],
            signature: String::new(),
        };
        auth = app_state.signer.sign(auth);
        let result = app_state.validate(Role::admin, auth).await;
        match result {
            Result::Unauthorized => { /* OKK */ }
            Result::Ok(_) => panic!("Expected unauthorized, but got authorized"),
        }
    }
}
