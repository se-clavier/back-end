use api::{
    Auth, Role, UserFull, UserFulls, UserSetRequest, UserSetResponse, UserSetValue,
    UsersListRequest, UsersListResponse,
};
use sqlx::{types::Json, QueryBuilder};

use super::AppState;

pub trait AdminAPI {
    async fn user_set(&self, req: UserSetRequest, auth: Auth) -> UserSetResponse;
    async fn users_list(&self, req: UsersListRequest, auth: Auth) -> UsersListResponse;
}

impl AdminAPI for AppState {
    async fn user_set(&self, req: UserSetRequest, _auth: Auth) -> UserSetResponse {
        let mut tx = self.database_pool.begin().await.unwrap();

        match req.operation {
            UserSetValue::delete => {
                sqlx::query("DELETE FROM user_roles WHERE user_id = ?")
                    .bind(req.user_id as i64)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                sqlx::query(
                    "UPDATE spares
                    SET assignee = NULL
                    WHERE assignee = ?",
                )
                .bind(req.user_id as i64)
                .execute(&mut *tx)
                .await
                .unwrap();
                sqlx::query("DELETE FROM users WHERE id = ?")
                    .bind(req.user_id as i64)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            }
            UserSetValue::roles(roles) => {
                sqlx::query("DELETE FROM user_roles WHERE user_id = ?")
                    .bind(req.user_id as i64)
                    .execute(&mut *tx)
                    .await
                    .unwrap();

                let mut roles_qb = QueryBuilder::new("INSERT INTO user_roles (user_id, role_type)");

                roles_qb.push_values(roles.into_iter(), |mut b, role| {
                    b.push_bind(req.user_id as i64).push_bind(role);
                });

                roles_qb.build().execute(&mut *tx).await.unwrap();
            }
            UserSetValue::password(password) => {
                sqlx::query("UPDATE users SET password = ? WHERE id = ?")
                    .bind(self.password_hasher.hash(&password))
                    .bind(req.user_id as i64)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            }
        }

        tx.commit().await.unwrap();

        UserSetResponse::Success
    }

    async fn users_list(&self, _req: UsersListRequest, _auth: Auth) -> UsersListResponse {
        let mut tx = self.database_pool.begin().await.unwrap();

        let users: UserFulls = sqlx::query_as(
            "
            SELECT id, username,
                json_group_array(json_object('type', user_roles.role_type)) 
                    FROM users
                    JOIN user_roles ON user_roles.user_id = users.id
                    GROUP BY id
            ",
        )
        .fetch_all(&mut *tx)
        .await
        .unwrap()
        .into_iter()
        .map(
            |(id, username, roles): (u64, String, Json<Vec<Role>>)| UserFull {
                id,
                username,
                roles: roles.as_ref().clone(),
            },
        )
        .collect();

        tx.commit().await.unwrap();

        UsersListResponse { users }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::app::test::TestApp;

    use api::{LoginRequest, LoginResponse, RevAPI};
    use sqlx::SqlitePool;

    #[sqlx::test(fixtures("users"))]
    fn test_users_list(pool: SqlitePool) {
        // Create a new test app instance
        let app = TestApp::new(pool);

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

        let res = app.users_list(UsersListRequest {}, auth.clone()).await;

        assert_eq!(
            res.users,
            vec![
                UserFull {
                    id: 1,
                    username: String::from("testuser"),
                    roles: vec![Role::user],
                },
                UserFull {
                    id: 2,
                    username: String::from("testadmin"),
                    roles: vec![Role::admin, Role::user, Role::terminal],
                },
            ]
        )
    }

    #[sqlx::test(fixtures("users"))]
    fn test_users_set_delete(pool: SqlitePool) {
        // Create a new test app instance
        let app = TestApp::new(pool);

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
            .user_set(
                UserSetRequest {
                    user_id: 1,
                    operation: UserSetValue::delete,
                },
                auth.clone(),
            )
            .await;

        assert_eq!(res, UserSetResponse::Success);

        let res = app.users_list(UsersListRequest {}, auth.clone()).await;

        assert_eq!(
            res.users,
            vec![UserFull {
                id: 2,
                username: String::from("testadmin"),
                roles: vec![Role::admin, Role::user, Role::terminal],
            },]
        )
    }

    #[sqlx::test(fixtures("users"))]
    fn test_users_set_roles(pool: SqlitePool) {
        // Create a new test app instance
        let app = TestApp::new(pool);

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
            .user_set(
                UserSetRequest {
                    user_id: 1,
                    operation: UserSetValue::roles(vec![Role::admin]),
                },
                auth.clone(),
            )
            .await;

        assert_eq!(res, UserSetResponse::Success);

        let res = app.users_list(UsersListRequest {}, auth.clone()).await;

        assert_eq!(
            res.users,
            vec![
                UserFull {
                    id: 1,
                    username: String::from("testuser"),
                    roles: vec![Role::admin],
                },
                UserFull {
                    id: 2,
                    username: String::from("testadmin"),
                    roles: vec![Role::admin, Role::user, Role::terminal],
                },
            ]
        )
    }

    #[sqlx::test(fixtures("users"))]
    fn test_users_set_password(pool: SqlitePool) {
        // Create a new test app instance
        let app = TestApp::new(pool);

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
            .user_set(
                UserSetRequest {
                    user_id: 1,
                    operation: UserSetValue::password(String::from("reset_password123")),
                },
                auth.clone(),
            )
            .await;

        assert_eq!(res, UserSetResponse::Success);

        app.check_reset("testuser", "reset_password123", "password123")
            .await;
    }
}
