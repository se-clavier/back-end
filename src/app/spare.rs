use super::AppState;
use api::*;
use chrono::Utc;
use sqlx::{query, Row};

pub trait SpareAPI {
    async fn spare_questionaire(
        &self,
        req: SpareQuestionaireRequest,
        auth: Auth,
    ) -> SpareQuestionaireResponse;
    async fn spare_return(&self, req: SpareReturnRequest, auth: Auth) -> SpareReturnResponse;
    async fn spare_take(&self, req: SpareTakeRequest, auth: Auth) -> SpareTakeResponse;
    async fn spare_list(&self, req: SpareListRequest, auth: Auth) -> SpareListResponse;
}

#[allow(unused)]
impl SpareAPI for AppState {

    async fn spare_questionaire(
        &self,
        req: SpareQuestionaireRequest,
        _auth: Auth,
    ) -> SpareQuestionaireResponse {
        let submitted_at = Utc::now().timestamp();

        for vacancy in req.vacancy.into_iter() {
            let available = matches!(vacancy, Vacancy::Available);
            query(
                r#"
                INSERT INTO vacancies (available, submitted_at)
                VALUES (?, ?)
                "#,
            )
            .bind(available)
            .bind(submitted_at)
            .execute(&self.database_pool)
            .await
            .unwrap();
        }

        SpareQuestionaireResponse::Success
    }


    async fn spare_take(&self, req: SpareTakeRequest, auth: Auth) -> SpareTakeResponse {
        let now = Utc::now().timestamp();
        let result = query(
            r#"
            UPDATE spares
               SET assignee    = ?,
                   taken_at    = ?,
                   returned_at = NULL
             WHERE id = ?
            "#,
        )
        .bind(auth.id as i64)
        .bind(now)
        .bind(req.id as i64)
        .execute(&self.database_pool)
        .await
        .unwrap();

        if result.rows_affected() == 0 {
            tracing::warn!("spare_take: no spares record with id {}", req.id);
        }

        SpareTakeResponse {}
    }

    async fn spare_return(
        &self,
        req: SpareReturnRequest,
        _auth: Auth,
    ) -> SpareReturnResponse {
        let now = Utc::now().timestamp();
        let result = query(
            r#"
            UPDATE spares
               SET returned_at = ?
             WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(req.id as i64)
        .execute(&self.database_pool)
        .await
        .unwrap();

        if result.rows_affected() == 0 {
            tracing::warn!("spare_return: no spares record with id {}", req.id);
        }

        SpareReturnResponse {}
    }

    async fn spare_list(
        &self,
        req: SpareListRequest,
        _auth: Auth,
    ) -> SpareListResponse {
        let room_rows = query("SELECT name FROM rooms ORDER BY id")
            .fetch_all(&self.database_pool)
            .await
            .unwrap();
        let rooms: Vec<Room> = room_rows
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        let week_no = match req {
            SpareListRequest::Schedule => (Utc::now().timestamp() / 604_800) as u64,
            SpareListRequest::Week(ts) => (ts / 604_800) as u64,
        };

        let spare_rows = query(
            r#"
            SELECT
                s.id                      AS id,
                s.stamp                   AS stamp,
                s.week                    AS week,
                s.taken_at                AS begin_time,
                COALESCE(s.returned_at, s.taken_at) AS end_time,
                r.name                    AS room,
                s.assignee                AS assignee_id,
                u.username                AS username
              FROM spares s
              JOIN rooms r ON s.room_id = r.id
         LEFT JOIN users u ON s.assignee = u.id
             WHERE s.week = ?
             ORDER BY s.id
            "#,
        )
        .bind(week_no as i64)
        .fetch_all(&self.database_pool)
        .await
        .unwrap();

        let spares: Vec<Spare> = spare_rows
            .into_iter()
            .map(|row| {
                let id = row.get::<i64, _>("id") as u64;
                let stamp = row.get::<i64, _>("stamp") as u64;
                let week = row.get::<i64, _>("week") as u64;
                let begin_time = row.get::<i64, _>("begin_time") as u64;
                let end_time = row.get::<i64, _>("end_time") as u64;
                let room = row.get::<String, _>("room");
                let assignee = row
                    .get::<Option<i64>, _>("assignee_id")
                    .and_then(|uid| {
                        row.get::<Option<String>, _>("username")
                            .map(|uname| User { id: uid as u64, username: uname })
                    });

                Spare {
                    id,
                    stamp,
                    week,
                    begin_time,
                    end_time,
                    room,
                    assignee,
                }
            })
            .collect();

        SpareListResponse { rooms, spares }
    }
}


#[cfg(test)]
#[allow(unused)]
mod test {
    use super::*;
    use crate::app::test::TestApp;
    use crate::app::sign::Signer;
    use api::{
        APICollection, Authed, Auth, Role,
        SpareQuestionaireRequest, SpareQuestionaireResponse,
        SpareTakeRequest, SpareTakeResponse,
        SpareReturnRequest, SpareReturnResponse,
        SpareListRequest, SpareListResponse,
        Vacancy,
    };
    use chrono::Utc;
    use sqlx::{query, Row, SqlitePool};

    #[sqlx::test]
    async fn test_spare_questionaire(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        let auth = Signer::default().sign(Auth {
            id: 1,
            roles: vec![Role::user],
            signature: "".into(),
        });

        let vacancies = vec![Vacancy::Available, Vacancy::Unavailable, Vacancy::Available];
        let req = SpareQuestionaireRequest { vacancy: vacancies.clone() };
        let resp: SpareQuestionaireResponse = app
            .request(APICollection::spare_questionaire(Authed { auth: auth.clone(), req }))
            .await;
        assert_eq!(resp, SpareQuestionaireResponse::Success);

        let rows = query("SELECT available FROM vacancies ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].get::<bool,_>("available"), true);
        assert_eq!(rows[1].get::<bool,_>("available"), false);
        assert_eq!(rows[2].get::<bool,_>("available"), true);
    }

    #[sqlx::test]
    async fn test_spare_take(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        query("INSERT INTO rooms (name) VALUES (?)")
            .bind("R").execute(&pool).await.unwrap();
        let week = (Utc::now().timestamp() / 604_800) as i64;
        query("INSERT INTO spares (room_id, stamp, taken_at, week) VALUES (?, ?, ?, ?)")
            .bind(1i64).bind(0i64).bind(0i64).bind(week)
            .execute(&pool).await.unwrap();

        let auth = Signer::default().sign(Auth {
            id: 42,
            roles: vec![Role::user],
            signature: "".into(),
        });
        let _: SpareTakeResponse = app
            .request(APICollection::spare_take(Authed {
                auth: auth.clone(),
                req: SpareTakeRequest { id: 1 },
            }))
            .await;

        let rec = query("SELECT assignee, returned_at FROM spares WHERE id = 1")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(rec.get::<Option<i64>,_>("assignee"), Some(42));
        assert!(rec.get::<Option<i64>,_>("returned_at").is_none());
    }

    #[sqlx::test]
    async fn test_spare_return(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        query("INSERT INTO rooms (name) VALUES (?)")
            .bind("R").execute(&pool).await.unwrap();
        let week = (Utc::now().timestamp() / 604_800) as i64;
        query("INSERT INTO spares (room_id, stamp, taken_at, week) VALUES (?, ?, ?, ?)")
            .bind(1i64).bind(0i64).bind(0i64).bind(week)
            .execute(&pool).await.unwrap();

        let auth = Signer::default().sign(Auth {
            id: 7,
            roles: vec![Role::user],
            signature: "".into(),
        });
        let _: SpareReturnResponse = app
            .request(APICollection::spare_return(Authed {
                auth: auth.clone(),
                req: SpareReturnRequest { id: 1 },
            }))
            .await;

        let rec = query("SELECT returned_at FROM spares WHERE id = 1")
            .fetch_one(&pool).await.unwrap();
        assert!(rec.get::<Option<i64>,_>("returned_at").is_some());
    }

    #[sqlx::test]
    async fn test_spare_list(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        query("INSERT INTO rooms (name) VALUES (?), (?)")
            .bind("X").bind("Y")
            .execute(&pool).await.unwrap();
        let week = (Utc::now().timestamp() / 604_800) as i64;
        query(
            "INSERT INTO spares (room_id, stamp, taken_at, returned_at, week)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(2i64).bind(5i64).bind(10i64).bind(20i64).bind(week)
        .execute(&pool).await.unwrap();

        let auth = Signer::default().sign(Auth {
            id: 3,
            roles: vec![Role::user],
            signature: "".into(),
        });
        let list: SpareListResponse = app
            .request(APICollection::spare_list(Authed {
                auth: auth.clone(),
                req: SpareListRequest::Schedule,
            }))
            .await;

        assert_eq!(list.rooms, vec!["X".to_string(), "Y".to_string()]);
        assert_eq!(list.spares.len(), 1);
        let sp = &list.spares[0];
        assert_eq!(sp.id, 1);
        assert_eq!(sp.stamp, 5);
        assert_eq!(sp.begin_time, 10);
        assert_eq!(sp.end_time, 20);
        assert_eq!(sp.room, "Y".to_string());
        assert!(sp.assignee.is_none());
    }
}
