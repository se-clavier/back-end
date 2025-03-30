use api::{Auth, API};
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

impl AppState {
    pub async fn new(db_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Create a new SQLite database if it doesn't exist
        if !Sqlite::database_exists(db_url).await? {
            Sqlite::create_database(db_url).await?;
        }

        // Create a connection pool to the SQLite database
        let pool = SqlitePool::connect(db_url).await?;

        // Run migrations
        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
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
        let roles: Vec<(i64, String)> =
            match sqlx::query_as("SELECT user_id, role_type FROM user_roles WHERE user_id = ?")
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
                .map(|(_, role)| match role.as_str() {
                    "admin" => api::Role::admin,
                    "user" => api::Role::user,
                    _ => api::Role::user,
                })
                .collect(),
        })
    }
    async fn register(&mut self, req: api::RegisterRequest) -> api::RegisterResponse {
        let id = sqlx::query("INSERT INTO users (username, password) VALUES (?, ?)")
            .bind(req.username)
            .bind(req.password)
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
}

#[cfg(test)]
mod tests {
    use api::API;

    #[tokio::test]
    async fn test_login() {
        let mut app = super::AppState::new("sqlite::memory:").await.unwrap();
        
        // Register a test user
        let req = api::RegisterRequest {
            username: String::from("testuser"),
            password: String::from("testpassword"),
        };

        match app.register(req).await {
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
        let req = api::LoginRequest {
            username: String::from("testuser"),
            password: String::from("testpassword"),
        };

        match app.login(req).await {
            api::LoginResponse::Success(auth) => {
                assert_eq!(auth.id, 1);
                assert_eq!(auth.signature, "signature");
                if auth.roles != vec![api::Role::user] {
                    panic!("unexpected roles");
                }
            }
            _ => panic!("login failed"),
        }
    }
}
