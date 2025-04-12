mod hash;
mod sign;
mod user;
use std::sync::Arc;

use api::{APICollection, API};
use axum::{extract::State, response::Response, routing::post, Json, Router};
use hash::Hasher;
use serde::Serialize;
use sign::Signer;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::subscriber::DefaultGuard;
use user::UserAPI;

#[derive(Debug, Clone)]
/// Application state
struct AppState {
    /// SQLite connection pool
    /// This pool is used to access the SQLite database
    database_pool: SqlitePool,
    password_hasher: Hasher,
    signer: Signer,
    _traing_guard: Arc<DefaultGuard>,
}

/// Handler for the root path
async fn handler(
    State(app): State<AppState>,
    Json(body): Json<APICollection>,
) -> Result<Json<impl Serialize>, Response> {
    Ok(Json(app.handle(body).await))
}

/// Create a new Axum router with the given pool
pub fn app(pool: SqlitePool, salt: &str, secret: &str, guard: DefaultGuard) -> Router {
    Router::new()
        .route("/", post(handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(AppState {
            database_pool: pool,
            password_hasher: Hasher::new(salt),
            signer: Signer::new(secret),
            _traing_guard: Arc::new(guard),
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

    async fn reset_password_admin(
        &self,
        req: api::ResetPasswordAdminRequest,
        auth: api::Auth,
    ) -> api::ResetPasswordAdminResponse {
        UserAPI::reset_password_admin(self, req, auth).await
    }
}

#[cfg(test)]
pub mod test {
    use super::{hash::test::TEST_SALT, sign::test::TEST_SECRET, *};
    use api::{Auth, Authed, Role, TestAuthEchoRequest, TestAuthEchoResponse};
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
        routing::RouterIntoService,
    };
    use http_body_util::BodyExt;
    use serde::de::DeserializeOwned;
    use std::{cell::RefCell, fmt::Debug};
    use tower::{Service, ServiceExt};
    use tracing_subscriber::util::SubscriberInitExt;

    pub struct TestRouter(RefCell<RouterIntoService<Body>>);
    
    impl From<Router> for TestRouter {
        fn from(value: Router) -> Self {
            Self(RefCell::new(value.into_service()))
        }
    }
    /// Test Helper trait for the app
    pub trait TestHelper {
        /// Test helper function that makes a request to router
        async fn test_request<T: DeserializeOwned + Debug>(&self, req: APICollection) -> T;

        /// Test helper function that check auth validate
        /// # Panics
        /// Invalid `auth` will panic with `"Check Auth Failed"`
        async fn test_check_auth(&self, auth: Auth) {
            let res: TestAuthEchoResponse = self
                .test_request(APICollection::test_auth_echo(Authed {
                    auth,
                    req: TestAuthEchoRequest {
                        data: "Check Validate".to_string(),
                    },
                }))
                .await;
            assert_eq!(res.data, "Check Validate", "Check Auth Failed");
        }
    }

    impl TestHelper for TestRouter {
        async fn test_request<T: DeserializeOwned + Debug>(&self, req: APICollection) -> T {
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
    
    impl API for TestRouter {
        async fn login(&self, req: api::LoginRequest) -> api::LoginResponse {
            self.test_request(APICollection::login(req)).await
        }
        async fn register(&self, req: api::RegisterRequest) -> api::RegisterResponse {
            self.test_request(APICollection::register(req)).await
        }
        async fn get_user(&self, req: api::Id) -> api::User {
            self.test_request(APICollection::get_user(req)).await
        }
        async fn test_auth_echo(
            &self,
            req: api::TestAuthEchoRequest,
            auth: api::Auth,
        ) -> api::TestAuthEchoResponse {
            self.test_request(APICollection::test_auth_echo(Authed { auth, req }))
                .await
        }
        async fn reset_password(
            &self,
            req: api::ResetPasswordRequest,
            auth: Auth,
        ) -> api::ResetPasswordResponse {
            self.test_request(APICollection::reset_password(Authed { auth, req }))
                .await
        }
        async fn reset_password_admin(
            &self,
            req: api::ResetPasswordAdminRequest,
            auth: Auth,
        ) -> api::ResetPasswordAdminResponse {
            self.test_request(APICollection::reset_password_admin(Authed { auth, req }))
                .await
        }
    }

    /// Test helper function that generates a new app instance
    pub async fn test_app(pool: SqlitePool) -> TestRouter {
        // Create a new app instance
        app(
            pool,
            TEST_SALT,
            TEST_SECRET,
            tracing_subscriber::fmt().with_test_writer().set_default(),
        )
        .into()
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
        let app = test_app(pool).await;

        let signer = Signer::new(TEST_SECRET);

        let auth = signer.sign(Auth {
            id: 1,
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
        let app = test_app(pool).await;

        let auth = Auth {
            id: 1,
            roles: vec![Role::admin],
            signature: "bad signature".to_string(),
        };

        let req = TestAuthEchoRequest {
            data: "Hacker Comes In".to_string(),
        };

        let res = app.test_auth_echo(req, auth).await;

        panic!("invalid auth check failed: {:?}", res);
    }
}
