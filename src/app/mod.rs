mod hash;

mod sign;

mod spare;

mod user;

mod config;

use api::{APICollection, API};
use axum::{extract::State, response::Response, routing::post, Json, Router};
use config::Config;
use hash::Hasher;
use serde::Serialize;
use sign::Signer;
use spare::SpareAPI;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use user::UserAPI;

const DEFAULT_SECRET: &str = "mysecret";

const DEFAULT_SALT: &str = "YmFzZXNhbHQ";

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
pub fn app(pool: SqlitePool, cfg: Config) -> Router {
    Router::new()
        .route("/", post(handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(AppState {
            database_pool: pool,
            password_hasher: Hasher::new(&cfg.salt),
            signer: Signer::new(&cfg.secret),
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

    async fn reset_password(
        &self,
        req: api::ResetPasswordRequest,
        auth: api::Auth,
    ) -> api::ResetPasswordResponse {
        UserAPI::reset_password(self, req, auth).await
    }

    async fn spare_questionaire(
        &self,
        req: api::SpareQuestionaireRequest,
        auth: api::Auth,
    ) -> api::SpareQuestionaireResponse {
        SpareAPI::spare_questionaire(self, req, auth).await
    }

    async fn spare_return(
        &self,
        req: api::SpareReturnRequest,
        auth: api::Auth,
    ) -> api::SpareReturnResponse {
        SpareAPI::spare_return(self, req, auth).await
    }

    async fn spare_take(
        &self,
        req: api::SpareTakeRequest,
        auth: api::Auth,
    ) -> api::SpareTakeResponse {
        SpareAPI::spare_take(self, req, auth).await
    }

    async fn spare_list(
        &self,
        req: api::SpareListRequest,
        auth: api::Auth,
    ) -> api::SpareListResponse {
        SpareAPI::spare_list(self, req, auth).await
    }
    async fn spare_init(
        &self,
        req: api::SpareInitRequest,
        auth: api::Auth,
    ) -> api::SpareInitResponse {
        SpareAPI::spare_init(self, req, auth).await
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use api::*;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
        routing::RouterIntoService,
    };
    use chrono::{TimeDelta, Utc};
    use http_body_util::BodyExt;
    use serde::de::DeserializeOwned;
    use std::{cell::RefCell, fmt::Debug};
    use tower::{Service, ServiceExt};
    use tracing::subscriber::DefaultGuard;
    use tracing_subscriber::util::SubscriberInitExt;

    #[allow(unused)]
    pub struct TestApp(RefCell<RouterIntoService<Body>>, DefaultGuard);

    impl From<Router> for TestApp {
        fn from(value: Router) -> Self {
            Self(
                RefCell::new(value.into_service()),
                tracing_subscriber::fmt().with_test_writer().set_default(),
            )
        }
    }

    impl TestApp {
        pub fn new(pool: SqlitePool) -> Self {
            app(pool, Default::default()).into()
        }

        /// Test helper function that check auth validate
        /// # Panics
        /// Invalid `auth` will panic with `"Check Auth Failed"`
        pub async fn check_auth(&self, auth: Auth) {
            let res = self
                .test_auth_echo(
                    TestAuthEchoRequest {
                        data: "Check Validate".to_string(),
                    },
                    auth,
                )
                .await;
            assert_eq!(res.data, "Check Validate", "Check Auth Failed");
        }
    }

    impl RevAPI for TestApp {
        async fn request<T: DeserializeOwned + Debug>(&self, req: APICollection) -> T {
            let res = self
                .0
                .borrow_mut()
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
        let app = TestApp::new(pool);

        let signer = Signer::default();

        let auth = signer.sign(Auth {
            id: 1,
            expire: (Utc::now() + TimeDelta::days(1)).to_rfc3339(),
            roles: vec![Role::user],
            signature: String::new(),
        });

        let req = TestAuthEchoRequest {
            data: "Hello, world!".to_string(),
        };

        let res = app.test_auth_echo(req, auth).await;

        assert_eq!(res.data, "Hello, world!");
    }

    #[sqlx::test]
    #[should_panic(expected = "request failed: Unauthorized")]
    async fn test_test_auth_echo_invalid(pool: SqlitePool) {
        // Create a new test app instance
        let app = TestApp::new(pool);

        let auth = Auth {
            id: 1,
            expire: (Utc::now() + TimeDelta::days(1)).to_rfc3339(),
            roles: vec![Role::admin],
            signature: "bad signature".to_string(),
        };

        let req = TestAuthEchoRequest {
            data: "Hacker Comes In".to_string(),
        };

        let res = app.test_auth_echo(req, auth).await;

        panic!("invalid auth check failed: {:?}", res);
    }

    #[sqlx::test]
    #[should_panic(expected = "request failed: Unauthorized")]
    async fn test_test_auth_echo_expired(pool: SqlitePool) {
        // Create a new test app instance
        let app = TestApp::new(pool);

        let signer = Signer::default();

        let auth = signer.sign(Auth {
            id: 1,
            expire: (Utc::now() + TimeDelta::days(-1)).to_rfc3339(),
            roles: vec![Role::user],
            signature: String::new(),
        });

        let req = TestAuthEchoRequest {
            data: "Hacker Comes In".to_string(),
        };

        let res = app.test_auth_echo(req, auth).await;

        panic!("invalid auth check failed: {:?}", res);
    }
}
