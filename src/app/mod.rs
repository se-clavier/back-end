mod hash;
mod sign;
mod user;
use api::{APICollection, API};
use axum::{extract::State, response::Response, routing::post, Json, Router};
use hash::Hasher;
use serde::Serialize;
use sign::Signer;
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
    signer: Signer,
}

/// Handler for the root path
async fn handler(
    State(app): State<AppState>,
    Json(body): Json<APICollection>,
) -> Result<Json<impl Serialize>, Response> {
    Ok(Json(app.handle(body).await))
}

/// Create a new Axum router with the given pool
pub fn app(pool: SqlitePool, salt: &str, secret: &str) -> Router {
    Router::new()
        .route("/", post(handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(AppState {
            database_pool: pool,
            password_hasher: Hasher::new(salt),
            signer: Signer::new(secret),
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
        req: api::TestAuthEchoRequest,
        _auth: api::Auth,
    ) -> api::TestAuthEchoResponse {
        api::TestAuthEchoResponse { data: req.data }
    }

    async fn validate(&self, role: api::Role, auth: api::Auth) -> api::Result<api::Auth> {
        self.signer.validate(role, auth)
    }
}

#[cfg(test)]
pub mod test {
    use super::{hash::test::TEST_SALT, sign::test::TEST_SECRET, *};
    use api::{Auth, Role, TestAuthEchoRequest, TestAuthEchoResponse};
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
        routing::RouterIntoService,
    };
    use http_body_util::BodyExt;
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;
    use tower::{Service, ServiceExt};
    use tracing::subscriber::DefaultGuard;
    use tracing_subscriber::util::SubscriberInitExt;

    /// Test Helper trait for the app
    pub trait TestHelper {
        /// Test helper function that makes a request to router
        async fn test_request<T: DeserializeOwned + Debug>(&mut self, req: APICollection) -> T;

        /// Test helper function that check auth validate
        /// # Panics
        /// Invalid `auth` will panic with `"Check Auth Failed"`
        async fn test_check_auth(&mut self, auth: api::Auth) {
            let res: TestAuthEchoResponse = self
                .test_request(APICollection::test_auth_echo(api::Authed {
                    auth: auth,
                    req: TestAuthEchoRequest {
                        data: "Check Validate".to_string(),
                    },
                }))
                .await;
            assert_eq!(res.data, "Check Validate", "Check Auth Failed");
        }
    }

    impl TestHelper for RouterIntoService<Body> {
        async fn test_request<T: DeserializeOwned + Debug>(&mut self, req: APICollection) -> T {
            let res = self
                .ready()
                .await
                .unwrap()
                .call(
                    Request::builder()
                        .method(http::Method::POST)
                        .uri("/")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&req).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
            let res: api::Result<T> =
                serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            match res {
                api::Result::Ok(res) => res,
                _ => panic!("request failed: {:?}", res),
            }
        }
    }

    /// Test helper function that generates a new app instance
    pub async fn test_app(pool: SqlitePool) -> (Router, DefaultGuard) {
        // Create a new tracing subscriber
        // This is used to log the test output
        let guard: DefaultGuard = tracing_subscriber::fmt().with_test_writer().set_default();

        // Create a new app instance
        let app = app(pool, TEST_SALT, TEST_SECRET);
        (app, guard)
    }

    #[tokio::test]
    async fn test_connect_pool() {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        connect_pool("sqlite::memory:").await;
    }

    #[sqlx::test]
    async fn test_test_auth_echo_valid(pool: SqlitePool) {
        // Create a new test app instance
        let (app, _guard) = test_app(pool).await;
        let mut app = app.into_service();

        let signer = Signer::new(TEST_SECRET);

        let auth = signer.sign(Auth {
            id: 1,
            roles: vec![Role::user],
            signature: String::new(),
        });

        let req = TestAuthEchoRequest {
            data: "Hello, world!".to_string(),
        };

        let res: TestAuthEchoResponse = app
            .test_request(APICollection::test_auth_echo(api::Authed { auth, req }))
            .await;

        assert_eq!(res.data, "Hello, world!");
    }

    #[sqlx::test]
    #[should_panic(expected = "request failed: Unauthorized")]
    async fn test_test_auth_echo_invalid(pool: SqlitePool) {
        // Create a new test app instance
        let (app, _guard) = test_app(pool).await;
        let mut app = app.into_service();

        let auth = Auth {
            id: 1,
            roles: vec![Role::admin],
            signature: "bad signature".to_string(),
        };

        let req = TestAuthEchoRequest {
            data: "Hacker Comes In".to_string(),
        };

        let res: TestAuthEchoResponse = app
            .test_request(APICollection::test_auth_echo(api::Authed { auth, req }))
            .await;
        
        panic!("invalid auth check failed: {:?}", res);
    }
}
