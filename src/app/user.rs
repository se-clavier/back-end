use api::{Auth, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse, Role};

use super::AppState;

pub trait UserAPI {
    async fn login(&self, req: LoginRequest) -> LoginResponse;
    async fn register(&self, req: RegisterRequest) -> RegisterResponse;
    async fn get_user(&self, req: api::Id) -> api::User;
}

impl UserAPI for AppState {
    /// login a user
    /// This function checks if the username and password are correct
    async fn login(&self, req: LoginRequest) -> LoginResponse {
        let user: (i64, String, String) =
            match sqlx::query_as("SELECT id, username, password FROM users WHERE username = ?")
                .bind(req.username)
                .fetch_one(&self.database_pool)
                .await
            {
                Ok(user) => user,
                Err(_) => return LoginResponse::FailureIncorrect,
            };

        // Check if the password is correct
        if !self
            .password_hasher
            .verify(req.password.as_str(), user.2.as_str())
        {
            tracing::info!("Incorrect password for user {:?}", (user.0, user.1));
            return LoginResponse::FailureIncorrect;
        }

        // Get the roles for the user
        let roles: Vec<(Role,)> =
            sqlx::query_as("SELECT role_type FROM user_roles WHERE user_id = ?")
                .bind(user.0)
                .fetch_all(&self.database_pool)
                .await
                .unwrap();

        tracing::info!(
            "User {:?} logged in with roles {:?}",
            (user.0, user.1),
            roles.iter().map(|(role,)| role).collect::<Vec<_>>()
        );

        LoginResponse::Success(Auth {
            id: user.0 as u64,
            signature: String::from("signature"),
            roles: roles.into_iter().map(|(role,)| role).collect(),
        })
    }

    /// Register a new user
    async fn register(&self, req: RegisterRequest) -> RegisterResponse {
        // Check if the username is already taken
        if sqlx::query("SELECT id FROM users WHERE username = ?")
            .bind(&req.username)
            .fetch_optional(&self.database_pool)
            .await
            .unwrap()
            .is_some()
        {
            return api::RegisterResponse::FailureUsernameTaken;
        }

        // Insert the user into the database
        let id = sqlx::query("INSERT INTO users (username, password) VALUES (?, ?)")
            .bind(&req.username)
            .bind(self.password_hasher.hash(&req.password))
            .execute(&self.database_pool)
            .await
            .unwrap()
            .last_insert_rowid();

        // Insert the user role into the database
        sqlx::query("INSERT INTO user_roles (user_id, role_type) VALUES (?, ?)")
            .bind(id)
            .bind(Role::user)
            .execute(&self.database_pool)
            .await
            .unwrap();

        tracing::info!("User {:?} registered", (id, req.username));

        api::RegisterResponse::Success(Auth {
            id: id as u64,
            signature: String::from("signature"),
            roles: vec![api::Role::user],
        })
    }

    /// Get user by ID
    async fn get_user(&self, req: api::Id) -> api::User {
        let (id, username): (u64, String) =
            sqlx::query_as("SELECT id, username FROM users WHERE id = ?")
                .bind(req as i64)
                .fetch_one(&self.database_pool)
                .await
                .unwrap();
        tracing::info!("User {:?} fetched", (id, &username));
        api::User { id, username }
    }
}

#[cfg(test)]
mod test {
    use crate::app;
    use crate::app::hash::test::TEST_SALT;

    use super::*;

    use api::APICollection;
    use axum::{
        body::{Body, Bytes},
        http::{self, Request, StatusCode},
        routing::RouterIntoService,
    };
    use http_body_util::BodyExt;
    use sqlx::SqlitePool;
    use tower::{Service, ServiceExt};
    use tracing_subscriber::util::SubscriberInitExt;

    /// Test helper function that makes a request to router
    async fn test_request(app: &mut RouterIntoService<Body>, req: APICollection) -> Bytes {
        let res = app
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
        res.into_body().collect().await.unwrap().to_bytes()
    }

    #[sqlx::test]
    /// Test the register API
    async fn test_register(pool: SqlitePool) {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        // Create a new app instance
        let mut app = app(pool, TEST_SALT).into_service();

        // Test register
        let body = test_request(
            &mut app,
            APICollection::register(RegisterRequest {
                username: String::from("testuser"),
                password: String::from("testpassword"),
            }),
        )
        .await;

        let res: api::Result<RegisterResponse> = serde_json::from_slice(&body).unwrap();
        match res {
            api::Result::Ok(RegisterResponse::Success(auth)) => {
                assert_eq!(auth.id, 1);
                assert_eq!(auth.signature, "signature");
                assert_eq!(auth.roles, vec![Role::user]);
            }
            _ => panic!("register failed"),
        }
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the register API with taken username
    /// This should return FailureUsernameTaken
    async fn test_register_username_taken(pool: SqlitePool) {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        // Create a new app instance
        let mut app = app(pool, TEST_SALT).into_service();

        // Test register
        let body = test_request(
            &mut app,
            APICollection::register(RegisterRequest {
                username: String::from("testuser"),
                password: String::from("testpassword"),
            }),
        )
        .await;

        let res: api::Result<RegisterResponse> = serde_json::from_slice(&body).unwrap();
        match res {
            api::Result::Ok(RegisterResponse::FailureUsernameTaken) => {}
            _ => panic!("username taken check failed"),
        }
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the login API
    async fn test_login_wrong_username(pool: SqlitePool) {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        // Create a new app instance
        let mut app = app(pool, TEST_SALT).into_service();

        // Test login with wrong username
        // This should return FailureIncorrect
        let body = test_request(
            &mut app,
            APICollection::login(LoginRequest {
                username: String::from("wrong_testuser"),
                password: String::from("password123"),
            }),
        )
        .await;

        let res: api::Result<LoginResponse> = serde_json::from_slice(&body).unwrap();
        match res {
            api::Result::Ok(LoginResponse::FailureIncorrect) => {}
            _ => panic!("wrong username check failed"),
        }
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the login API
    async fn test_login_wrong_password(pool: SqlitePool) {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        // Create a new app instance
        let mut app = app(pool, TEST_SALT).into_service();

        let body = test_request(
            &mut app,
            APICollection::login(LoginRequest {
                username: String::from("testuser"),
                password: String::from("wrong_password"),
            }),
        )
        .await;

        let res: api::Result<LoginResponse> = serde_json::from_slice(&body).unwrap();
        match res {
            api::Result::Ok(LoginResponse::FailureIncorrect) => {}
            _ => panic!("wrong password check failed"),
        }
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the login API
    async fn test_login_correct_password(pool: SqlitePool) {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        // Create a new app instance
        let mut app = app(pool, TEST_SALT).into_service();

        let body = test_request(
            &mut app,
            APICollection::login(LoginRequest {
                username: String::from("testuser"),
                password: String::from("password123"),
            }),
        )
        .await;

        let res: api::Result<LoginResponse> = serde_json::from_slice(&body).unwrap();

        match res {
            api::Result::Ok(LoginResponse::Success(auth)) => {
                assert_eq!(auth.id, 1);
                assert_eq!(auth.signature, "signature");
                assert_eq!(auth.roles, vec![Role::user]);
            }
            _ => panic!("login failed"),
        }
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the get_user API
    async fn test_get_user(pool: SqlitePool) {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        // Create a new app instance
        let mut app = app(pool, TEST_SALT).into_service();

        let body = test_request(&mut app, APICollection::get_user(1_u64)).await;

        let res: api::Result<api::User> = serde_json::from_slice(&body).unwrap();
        let res = match res {
            api::Result::Ok(res) => res,
            _ => panic!("get_user failed"),
        };

        assert_eq!(res.id, 1);
        assert_eq!(res.username, "testuser");
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the get_user API with not found user
    #[should_panic]
    async fn test_get_user_not_found(pool: SqlitePool) {
        // Create a new tracing subscriber
        // This is used to log the test output
        let _tracing_guard = tracing_subscriber::fmt().with_test_writer().set_default();

        // Create a new app instance
        let mut app = app(pool, TEST_SALT).into_service();

        let body = test_request(&mut app, APICollection::get_user(2_u64)).await;

        let res: api::Result<api::User> = serde_json::from_slice(&body).unwrap();
        let res = match res {
            api::Result::Ok(res) => res,
            _ => panic!("get_user failed"),
        };

        assert_eq!(res.id, 1);
        assert_eq!(res.username, "testuser");
    }
}
