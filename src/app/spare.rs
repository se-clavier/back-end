use super::AppState;
use api::{Auth, SpareInitRequest, SpareInitResponse, SpareListRequest, SpareListResponse};
use api::{
    Room, Spare, SpareQuestionaireRequest, SpareQuestionaireResponse, SpareReturnRequest,
    SpareReturnResponse, SpareTakeRequest, SpareTakeResponse, User, Vacancy,
};
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
    async fn spare_init(&self, req: SpareInitRequest, auth: Auth) -> SpareInitResponse;
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

    async fn spare_return(&self, req: SpareReturnRequest, _auth: Auth) -> SpareReturnResponse {
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

    async fn spare_list(&self, req: SpareListRequest, _auth: Auth) -> SpareListResponse {
        let rooms: Vec<Room> = query("SELECT name FROM rooms ORDER BY id")
            .fetch_all(&self.database_pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        let week_no: u64 = match req {
            SpareListRequest::Schedule => (Utc::now().timestamp() as u64) / 604_800,
            SpareListRequest::Week(ts_str) => {
                let ts = ts_str.parse::<u64>().unwrap_or(0);
                ts / 604_800
            }
        };

        let rows = query(
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
			  JOIN rooms r   ON s.room_id  = r.id
		 LEFT JOIN users u   ON s.assignee = u.id
			 WHERE s.week = ?
			 ORDER BY s.id
			"#,
        )
        .bind(week_no as i64)
        .fetch_all(&self.database_pool)
        .await
        .unwrap();

        let spares: Vec<Spare> = rows
            .into_iter()
            .map(|row| {
                let id = row.get::<i64, _>("id") as u64;
                let stamp = row.get::<i64, _>("stamp") as u64;
                let week_val = row.get::<i64, _>("week") as u64;
                let begin_time_val = row.get::<i64, _>("begin_time") as u64;
                let end_time_val = row.get::<i64, _>("end_time") as u64;
                let room = row.get::<String, _>("room");
                let assignee = row.get::<Option<i64>, _>("assignee_id").and_then(|uid| {
                    row.get::<Option<String>, _>("username").map(|uname| User {
                        id: uid as u64,
                        username: uname,
                    })
                });

                Spare {
                    id,
                    stamp,
                    week: week_val.to_string(),
                    begin_time: begin_time_val.to_string(),
                    end_time: end_time_val.to_string(),
                    room,
                    assignee,
                }
            })
            .collect();

        SpareListResponse { rooms, spares }
    }
    async fn spare_init(&self, req: SpareInitRequest, _auth: Auth) -> SpareInitResponse {
        query("DELETE FROM spares")
            .execute(&self.database_pool)
            .await
            .unwrap();
        query("DELETE FROM sqlite_sequence WHERE name='spares'")
            .execute(&self.database_pool)
            .await
            .unwrap();

        query("DELETE FROM rooms")
            .execute(&self.database_pool)
            .await
            .unwrap();
        query("DELETE FROM sqlite_sequence WHERE name='rooms'")
            .execute(&self.database_pool)
            .await
            .unwrap();

        for room in &req.rooms {
            query("INSERT INTO rooms (name) VALUES (?)")
                .bind(room)
                .execute(&self.database_pool)
                .await
                .unwrap();
        }

        for spare in &req.spares {
            let idx = req
                .rooms
                .iter()
                .position(|r| r == &spare.room)
                .expect("spare_init: room not found in request");
            let room_id = (idx + 1) as i64;
            let assignee = spare.assignee.as_ref().map(|u| u.id as i64);
            let week_ts = spare.week.parse::<i64>().unwrap_or(0);
            let begin_ts = spare.begin_time.parse::<i64>().unwrap_or(0);
            let end_ts = spare.end_time.parse::<i64>().unwrap_or(begin_ts);

            query(
                "INSERT INTO spares \
                 (room_id, stamp, taken_at, returned_at, week, assignee) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(room_id)
            .bind(spare.stamp as i64)
            .bind(begin_ts)
            .bind(Some(end_ts))
            .bind(week_ts)
            .bind(assignee)
            .execute(&self.database_pool)
            .await
            .unwrap();
        }

        SpareInitResponse::Success
    }
}

#[cfg(test)]
#[allow(unused)]
mod test {
    use super::*;
    use crate::app::sign::Signer;
    use crate::app::test::TestApp;
    use api::*;
    use chrono::Utc;
    use sqlx::{query, SqlitePool};

    #[sqlx::test]
    async fn test_spare_questionaire(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        let auth = Signer::default().sign(Auth {
            id: 1,
            roles: vec![Role::user],
            signature: "".into(),
        });
        let vacs = vec![Vacancy::Available, Vacancy::Unavailable, Vacancy::Available];
        let req = SpareQuestionaireRequest {
            vacancy: vacs.clone(),
        };
        let resp: SpareQuestionaireResponse = app
            .request(APICollection::spare_questionaire(Authed {
                auth: auth.clone(),
                req,
            }))
            .await;
        assert_eq!(resp, SpareQuestionaireResponse::Success);

        let rows = query("SELECT available FROM vacancies ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].get::<bool, _>("available"), true);
        assert_eq!(rows[1].get::<bool, _>("available"), false);
        assert_eq!(rows[2].get::<bool, _>("available"), true);
    }

    #[sqlx::test(fixtures("rooms"))]
    async fn test_spare_take(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        let week = (Utc::now().timestamp() / 604_800) as i64;
        query("INSERT INTO spares (room_id, stamp, taken_at, week) VALUES (1, 0, 0, ?)")
            .bind(week)
            .execute(&pool)
            .await
            .unwrap();

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
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rec.get::<Option<i64>, _>("assignee"), Some(42));
        assert!(rec.get::<Option<i64>, _>("returned_at").is_none());
    }

    #[sqlx::test(fixtures("rooms"))]
    async fn test_spare_return(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        let week = (Utc::now().timestamp() / 604_800) as i64;
        query("INSERT INTO spares (room_id, stamp, taken_at, week) VALUES (1, 0, 0, ?)")
            .bind(week)
            .execute(&pool)
            .await
            .unwrap();

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
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(rec.get::<Option<i64>, _>("returned_at").is_some());
    }

    #[sqlx::test(fixtures("rooms_xy", "spares_list"))]
    async fn test_spare_list(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        let auth = Signer::default().sign(Auth {
            id: 3,
            roles: vec![Role::user],
            signature: "".into(),
        });

        let list: SpareListResponse = app
            .request(APICollection::spare_list(Authed {
                auth: auth.clone(),
                req: SpareListRequest::Week("0".into()),
            }))
            .await;

        assert_eq!(list.rooms, vec![String::from("X"), String::from("Y")]);

        assert_eq!(list.spares.len(), 1);
        let sp = &list.spares[0];
        assert_eq!(sp.id, 1);
        assert_eq!(sp.stamp, 5);
        assert_eq!(sp.week, "0".to_string());
        assert_eq!(sp.begin_time, "10".to_string());
        assert_eq!(sp.end_time, "20".to_string());

        assert_eq!(sp.room, "Y".to_string());
        assert!(sp.assignee.is_none());
    }
    #[sqlx::test(fixtures("init_old_data"))]
    async fn test_spare_init(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        let auth = Signer::default().sign(Auth {
            id: 10,
            roles: vec![Role::admin],
            signature: "".into(),
        });

        let rooms = vec!["A".to_string(), "B".to_string()];
        let sp = Spare {
            id: 0,
            stamp: 100,
            week: "2".to_string(),
            begin_time: "15".to_string(),
            end_time: "25".to_string(),
            room: "B".to_string(),
            assignee: Some(User {
                id: 42,
                username: "u1".to_string(),
            }),
        };
        let req = SpareInitRequest {
            rooms: rooms.clone(),
            spares: vec![sp.clone()],
        };
        let resp: SpareInitResponse = app
            .request(APICollection::spare_init(Authed {
                auth: auth.clone(),
                req,
            }))
            .await;
        assert_eq!(resp, SpareInitResponse::Success);
        let got_rooms: Vec<String> = query("SELECT name FROM rooms ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.get::<String, _>("name"))
            .collect();
        assert_eq!(got_rooms, rooms);
        let row = query(
            r#"
        SELECT
          r.name                  AS room,
          s.stamp                 AS stamp,
          s.taken_at              AS taken_at,
          s.returned_at           AS returned_at,
          CAST(s.week AS TEXT)    AS week,
          s.assignee              AS assignee
        FROM spares s
        JOIN rooms r ON s.room_id = r.id
    "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.get::<String, _>("room"), "B");
        assert_eq!(row.get::<i64, _>("stamp"), 100);
        assert_eq!(row.get::<i64, _>("taken_at"), 15);
        assert_eq!(row.get::<i64, _>("returned_at"), 25);
        assert_eq!(row.get::<String, _>("week"), "2");
        assert_eq!(row.get::<Option<i64>, _>("assignee"), Some(42));
    }
}
