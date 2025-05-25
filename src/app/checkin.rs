use api::{
    Auth, CheckinRequest, CheckinResponse, CheckoutRequest, CheckoutResponse,
    TerminalCredentialRequest, TerminalCredentialResponse,
};
use chrono::{TimeDelta, Utc};

use crate::app::{parse_time_delta, parse_week, AppState};

pub trait CheckinAPI {
    async fn terminal_credential(
        &self,
        req: TerminalCredentialRequest,
        auth: Auth,
    ) -> TerminalCredentialResponse;
    async fn checkin(&self, req: CheckinRequest, auth: Auth) -> CheckinResponse;
    async fn checkout(&self, req: CheckoutRequest, auth: Auth) -> CheckoutResponse;
}

impl CheckinAPI for AppState {
    async fn checkin(&self, req: CheckinRequest, auth: Auth) -> CheckinResponse {
        match self.signer.validate(api::Role::terminal, req.credential) {
            api::Result::Ok(_) => {}
            _ => {
                return CheckinResponse::InvailidCredential;
            }
        }
        let mut tx = self.database_pool.begin().await.unwrap();
        let (checkin, begin_at, week): (Option<i64>, String, String) = sqlx::query_as(
            "SELECT checkin, begin_at, week from spares WHERE id = ? AND assignee = ?",
        )
        .bind(req.id as i64)
        .bind(auth.id as i64)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        let res = if checkin.is_none() {
            let begin_at = parse_week(week) + parse_time_delta(begin_at);
            let now = chrono::Utc::now();
            if now + TimeDelta::minutes(30) < begin_at {
                CheckinResponse::Early
            } else {
                let late = (now - begin_at).num_minutes();
                sqlx::query("UPDATE spares SET checkin = ? WHERE id = ?")
                    .bind(late)
                    .bind(req.id as i64)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                if late > 0 {
                    CheckinResponse::Late(late)
                } else {
                    CheckinResponse::Intime
                }
            }
        } else {
            CheckinResponse::Duplicate
        };

        tx.commit().await.unwrap();
        res
    }

    async fn checkout(&self, req: CheckoutRequest, auth: Auth) -> CheckoutResponse {
        match self.signer.validate(api::Role::terminal, req.credential) {
            api::Result::Ok(_) => {}
            _ => {
                return CheckoutResponse::InvailidCredential;
            }
        }
        let mut tx = self.database_pool.begin().await.unwrap();
        let (checkin, checkout, end_at, week): (Option<i64>, Option<i64>, String, String) =
            sqlx::query_as(
                "SELECT checkin, checkout, end_at, week from spares WHERE id = ? AND assignee = ?",
            )
            .bind(req.id as i64)
            .bind(auth.id as i64)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        let res = if checkin.is_none() {
            CheckoutResponse::NotCheckedIn
        } else if checkout.is_none() {
            let end_at = parse_week(week) + parse_time_delta(end_at);
            let now = chrono::Utc::now();
            if now + TimeDelta::minutes(30) < end_at {
                CheckoutResponse::Early
            } else if now > end_at + TimeDelta::minutes(30) {
                CheckoutResponse::Late
            } else {
                let early = (end_at - now).num_minutes();
                sqlx::query("UPDATE spares SET checkout = ? WHERE id = ?")
                    .bind(early)
                    .bind(req.id as i64)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                CheckoutResponse::Intime
            }
        } else {
            CheckoutResponse::Duplicate
        };

        tx.commit().await.unwrap();
        res
    }

    async fn terminal_credential(
        &self,
        _: TerminalCredentialRequest,
        auth: Auth,
    ) -> TerminalCredentialResponse {
        TerminalCredentialResponse {
            auth: self.signer.sign(Auth {
                expire: (Utc::now() + TimeDelta::minutes(5)).to_rfc3339(),
                ..auth
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use api::{LoginRequest, LoginResponse, RevAPI};
    use sqlx::SqlitePool;

    use super::*;
    use crate::app::test::TestApp;

    #[sqlx::test(fixtures("users"))]
    fn test_terminal_credential(pool: SqlitePool) {
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
        app.terminal_credential(TerminalCredentialRequest {}, auth)
            .await;
    }

    #[sqlx::test(fixtures("users"))]
    fn test_checkin_invalid_credential(pool: SqlitePool) {
        let app = TestApp::new(pool);
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
        let req = CheckinRequest {
            id: 1,
            credential: Auth {
                id: 1,
                expire: String::new(),
                roles: Vec::new(),
                signature: String::new(),
            },
        };
        assert_eq!(
            CheckinResponse::InvailidCredential,
            app.checkin(req, auth).await
        );
    }

    #[sqlx::test(fixtures("users"))]
    fn test_checkout_invalid_credential(pool: SqlitePool) {
        let app = TestApp::new(pool);
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
        let req = CheckoutRequest {
            id: 1,
            credential: Auth {
                id: 1,
                expire: String::new(),
                roles: Vec::new(),
                signature: String::new(),
            },
        };
        assert_eq!(
            CheckoutResponse::InvailidCredential,
            app.checkout(req, auth).await
        );
    }

    #[sqlx::test(fixtures("users", "spares"))]
    fn test_checkin(pool: SqlitePool) {
        let app = TestApp::new(pool);
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
        let credential = match app
            .login(LoginRequest {
                username: String::from("testadmin"),
                password: String::from("password123"),
            })
            .await
        {
            LoginResponse::Success(auth) => auth,
            _ => panic!("login failed"),
        };
        let req = CheckinRequest { id: 2, credential };
        let res = app.checkin(req, auth).await;
        match res {
            CheckinResponse::Late(_) => {}
            _ => panic!("checkin failed"),
        }
    }

    #[sqlx::test(fixtures("users", "spares"))]
    fn test_checkout(pool: SqlitePool) {
        let app = TestApp::new(pool);
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
        let credential = match app
            .login(LoginRequest {
                username: String::from("testadmin"),
                password: String::from("password123"),
            })
            .await
        {
            LoginResponse::Success(auth) => auth,
            _ => panic!("login failed"),
        };
        let req = CheckoutRequest { id: 4, credential };
        let res = app.checkout(req, auth).await;
        assert_eq!(res, CheckoutResponse::Late);
    }
}
