use api::{
    Auth, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse,
    ResetPasswordAdminRequest, ResetPasswordAdminResponse, ResetPasswordRequest,
    ResetPasswordResponse, Role,
};

use super::AppState;

pub trait UserAPI {
    async fn login(&self, req: LoginRequest) -> LoginResponse;
    async fn register(&self, req: RegisterRequest) -> RegisterResponse;
    async fn get_user(&self, req: api::Id) -> api::User;
    async fn reset_password(&self, req: ResetPasswordRequest, auth: Auth) -> ResetPasswordResponse;
    async fn reset_password_admin(
        &self,
        req: ResetPasswordAdminRequest,
        auth: Auth,
    ) -> ResetPasswordAdminResponse;
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

        LoginResponse::Success(self.signer.sign(Auth {
            id: user.0 as u64,
            signature: String::new(),
            roles: roles.into_iter().map(|(role,)| role).collect(),
        }))
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

        api::RegisterResponse::Success(self.signer.sign(Auth {
            id: id as u64,
            signature: String::new(),
            roles: vec![api::Role::user],
        }))
    }

    /// Get user by ID
    async fn get_user(&self, req: api::Id) -> api::User {
        let (id, username): (u64, String) =
            sqlx::query_as("SELECT id, username FROM users WHERE id = ?")
                .bind(req as i64)
                .fetch_optional(&self.database_pool)
                .await
                .unwrap()
                .expect("User not found");
        tracing::info!("User {:?} fetched", (id, &username));
        api::User { id, username }
    }

    async fn reset_password(
        &self,
        req: ResetPasswordRequest,
        auth: api::Auth,
    ) -> ResetPasswordResponse {
        sqlx::query("UPDATE users SET password = ? WHERE id = ?")
            .bind(self.password_hasher.hash(&req.password))
            .bind(auth.id as i64)
            .execute(&self.database_pool)
            .await
            .unwrap();
        tracing::info!("Password of user {:?} changed", auth.id);
        ResetPasswordResponse::Success
    }

    async fn reset_password_admin(
        &self,
        req: ResetPasswordAdminRequest,
        _auth: Auth,
    ) -> ResetPasswordAdminResponse {
        let res = sqlx::query("UPDATE users SET password = ? WHERE id = ?")
            .bind(self.password_hasher.hash(&req.password))
            .bind(req.id as i64)
            .execute(&self.database_pool)
            .await
            .unwrap();
        if res.rows_affected() == 0 {
            tracing::info!("User {:?} not found", req.id);
            return ResetPasswordAdminResponse::FailureUserNotFound;
        }
        tracing::info!("Password of user {:?} changed", req.id);
        ResetPasswordAdminResponse::Success
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::test::{test_app, TestHelper, TestRouter};
    use api::{APICollection, API};
    use sqlx::SqlitePool;

    #[sqlx::test]
    /// Test the register API
    async fn test_register(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;
        // Test register
        let res = app
            .register(RegisterRequest {
                username: String::from("testuser"),
                password: String::from("testpassword"),
            })
            .await;

        match res {
            RegisterResponse::Success(auth) => {
                assert_eq!(auth.id, 1);
                assert_eq!(auth.roles, vec![Role::user]);
                app.test_check_auth(auth).await;
            }
            _ => panic!("register failed"),
        }
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the register API with taken username
    /// This should return FailureUsernameTaken
    async fn test_register_username_taken(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;

        // Test register
        let res: RegisterResponse = app
            .register(RegisterRequest {
                username: String::from("testuser"),
                password: String::from("testpassword"),
            })
            .await;

        assert_eq!(
            res,
            RegisterResponse::FailureUsernameTaken,
            "username taken check failed"
        );
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the login API
    async fn test_login_wrong_username(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;

        // Test login with wrong username
        // This should return FailureIncorrect
        let res: LoginResponse = app
            .login(LoginRequest {
                username: String::from("wrong_testuser"),
                password: String::from("password123"),
            })
            .await;

        assert_eq!(
            res,
            LoginResponse::FailureIncorrect,
            "wrong username check failed"
        );
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the login API
    async fn test_login_wrong_password(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;

        // Test login with wrong password
        let res = app
            .login(LoginRequest {
                username: String::from("testuser"),
                password: String::from("wrong_password"),
            })
            .await;

        assert_eq!(
            res,
            LoginResponse::FailureIncorrect,
            "wrong password check failed"
        );
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the login API
    async fn test_login_correct_password(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;

        let res: LoginResponse = app
            .test_request(APICollection::login(LoginRequest {
                username: String::from("testuser"),
                password: String::from("password123"),
            }))
            .await;

        match res {
            LoginResponse::Success(auth) => {
                assert_eq!(auth.id, 1);
                assert_eq!(auth.roles, vec![Role::user]);
                app.test_check_auth(auth).await;
            }
            _ => panic!("login failed"),
        }
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the get_user API
    async fn test_get_user(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;

        let res = app.get_user(1).await;

        assert_eq!(res.id, 1);
        assert_eq!(res.username, "testuser");
    }

    #[sqlx::test(fixtures("users"))]
    /// Test the get_user API with not found user
    #[should_panic(expected = "User not found")]
    async fn test_get_user_not_found(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;

        let _res: api::User = app.get_user(404).await;
    }

    async fn check_reset(app: &TestRouter, username: &str, password: &str, chk_password: &str) {
        match app
            .login(LoginRequest {
                username: String::from(username),
                password: String::from(password),
            })
            .await
        {
            LoginResponse::Success(auth) => {
                assert_eq!(auth.id, 1);
                assert_eq!(auth.roles, vec![Role::user]);
                app.test_check_auth(auth).await;
            }
            _ => panic!("reset login failed"),
        }

        let res = app
            .login(LoginRequest {
                username: String::from(username),
                password: String::from(chk_password),
            })
            .await;

        assert_eq!(res, LoginResponse::FailureIncorrect, "reset check failed");
    }

    #[sqlx::test(fixtures("users"))]
    async fn test_reset_passwd(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;

        let auth = match app
            .login(LoginRequest {
                username: String::from("testuser"),
                password: String::from("password123"),
            })
            .await
        {
            LoginResponse::Success(auth) => auth,
            _ => panic!("login failed"),
        };

        let res = app
            .reset_password(
                ResetPasswordRequest {
                    password: String::from("reset_password123"),
                },
                auth,
            )
            .await;

        assert_eq!(res, ResetPasswordResponse::Success, "reset failed");

        check_reset(&app, "testuser", "reset_password123", "password123").await;
    }

    #[sqlx::test(fixtures("users"))]
    async fn test_reset_passwd_admin(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;

        let auth = match app
            .login(LoginRequest {
                username: String::from("testadmin"),
                password: String::from("password123"),
            })
            .await
        {
            LoginResponse::Success(auth) => auth,
            _ => panic!("login failed"),
        };

        let res = app
            .reset_password_admin(
                ResetPasswordAdminRequest {
                    id: 1,
                    password: String::from("reset_password123"),
                },
                auth,
            )
            .await;

        assert_eq!(res, ResetPasswordAdminResponse::Success, "reset failed");

        check_reset(&app, "testuser", "reset_password123", "password123").await;
    }

    #[sqlx::test(fixtures("users"))]
    async fn test_reset_passwd_admin_not_found(pool: SqlitePool) {
        // Create a new test app instance
        let app = test_app(pool).await;

        let auth = match app
            .login(LoginRequest {
                username: String::from("testadmin"),
                password: String::from("password123"),
            })
            .await
        {
            LoginResponse::Success(auth) => auth,
            _ => panic!("login failed"),
        };

        let res = app
            .reset_password_admin(
                ResetPasswordAdminRequest {
                    id: 404,
                    password: String::from("reset_password123"),
                },
                auth,
            )
            .await;

        assert_eq!(
            res,
            ResetPasswordAdminResponse::FailureUserNotFound,
            "not found check failed"
        );
    }
}
