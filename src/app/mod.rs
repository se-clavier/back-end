use api::{APICollection, Auth, API};
use axum::{extract::State, response::Response, routing::post, Json, Router};
use serde::Serialize;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
}

async fn handler(
    State(mut app): State<AppState>,
    Json(body): Json<APICollection>,
) -> Result<Json<impl Serialize>, Response> {
    Ok(Json(app.handle(body).await))
}

pub async fn app(db_url: &str) -> Router {
    Router::new()
        .route("/", post(handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(AppState::new(db_url).await)
}

impl AppState {
    async fn new(db_url: &str) -> Self {
        // Create a new SQLite database if it doesn't exist
        if !Sqlite::database_exists(db_url).await.unwrap() {
            Sqlite::create_database(db_url).await.unwrap();
        }

        // Create a connection pool to the SQLite database
        let pool = SqlitePool::connect(db_url).await.unwrap();

        // Run migrations
        sqlx::migrate!().run(&pool).await.unwrap();

        Self { pool }
    }
}

impl API for AppState {
    async fn login(&mut self, req: api::LoginRequest) -> api::LoginResponse {
        let user: (i64, String, String) =
            match sqlx::query_as("SELECT id, username, password FROM users WHERE username = ?")
                .bind(req.username)
                .fetch_one(&self.pool)
                .await
            {
                Ok(user) => user,
                Err(_) => return api::LoginResponse::FailureIncorrect,
            };
        if user.2 != req.password {
            return api::LoginResponse::FailureIncorrect;
        }
        let roles: Vec<(String,)> =
            match sqlx::query_as("SELECT role_type FROM user_roles WHERE user_id = ?")
                .bind(user.0)
                .fetch_all(&self.pool)
                .await
            {
                Ok(res) => res,
                Err(_) => return api::LoginResponse::FailureIncorrect,
            };

        api::LoginResponse::Success(Auth {
            id: user.0 as u64,
            signature: String::from("signature"),
            roles: roles
                .iter()
                .map(|(role,)| match role.as_str() {
                    "admin" => api::Role::admin,
                    "user" => api::Role::user,
                    _ => api::Role::user,
                })
                .collect(),
        })
    }
    async fn register(&mut self, req: api::RegisterRequest) -> api::RegisterResponse {
        if sqlx::query("SELECT id FROM users WHERE username = ?")
            .bind(&req.username)
            .fetch_optional(&self.pool)
            .await
            .unwrap()
            .is_some()
        {
            return api::RegisterResponse::FailureUsernameTaken;
        }
        let id = sqlx::query("INSERT INTO users (username, password) VALUES (?, ?)")
            .bind(&req.username)
            .bind(&req.password)
            .execute(&self.pool)
            .await
            .unwrap()
            .last_insert_rowid();
        sqlx::query("INSERT INTO user_roles (user_id, role_type) VALUES (?, ?)")
            .bind(id)
            .bind("user")
            .execute(&self.pool)
            .await
            .unwrap();
        api::RegisterResponse::Success(Auth {
            id: id as u64,
            signature: String::from("signature"),
            roles: vec![api::Role::user],
        })
    }
    async fn get_user(&mut self, req: api::Id) -> api::User {
        let (id, username): (u64, String) =
            sqlx::query_as("SELECT id, username FROM users WHERE id = ?")
                .bind(req as i64)
                .fetch_one(&self.pool)
                .await
                .unwrap();
        api::User { id, username }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde::Serialize;
    use tower::{Service, ServiceExt};

    fn test_request_json<T: Serialize>(req: &T) -> Request<Body> {
        Request::builder()
            .method(http::Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(req).unwrap()))
            .unwrap()
    }

    #[tokio::test]
    async fn test() {
        let mut app = super::app("sqlite::memory:").await.into_service();

        // Test register
        let res = app
            .ready()
            .await
            .unwrap()
            .call(test_request_json(&api::APICollection::register(
                api::RegisterRequest {
                    username: String::from("testuser"),
                    password: String::from("testpassword"),
                },
            )))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let res: api::Result<api::RegisterResponse> = serde_json::from_slice(&body).unwrap();
        let res = match res {
            api::Result::Ok(res) => res,
            _ => panic!("register failed"),
        };
        match res {
            api::RegisterResponse::Success(auth) => {
                assert_eq!(auth.id, 1);
                assert_eq!(auth.signature, "signature");
                if auth.roles != vec![api::Role::user] {
                    panic!("unexpected roles");
                }
            }
            _ => panic!("register failed"),
        }

        // Test login
        let res = app
            .ready()
            .await
            .unwrap()
            .call(test_request_json(&api::APICollection::login(
                api::LoginRequest {
                    username: String::from("testuser"),
                    password: String::from("testpassword"),
                },
            )))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let res: api::Result<api::LoginResponse> = serde_json::from_slice(&body).unwrap();
        let res = match res {
            api::Result::Ok(res) => res,
            _ => panic!("login failed"),
        };

        match res {
            api::LoginResponse::Success(auth) => {
                assert_eq!(auth.id, 1);
                assert_eq!(auth.signature, "signature");
                if auth.roles != vec![api::Role::user] {
                    panic!("unexpected roles");
                }
            }
            _ => panic!("login failed"),
        }

        // Test get user
        let res = app
            .ready()
            .await
            .unwrap()
            .call(test_request_json(&api::APICollection::get_user(1_u64)))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let res: api::Result<api::User> = serde_json::from_slice(&body).unwrap();
        let res = match res {
            api::Result::Ok(res) => res,
            _ => panic!("get_user failed"),
        };
        assert_eq!(res.id, 1);
        assert_eq!(res.username, "testuser");
    }
}
