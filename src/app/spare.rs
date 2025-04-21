use super::AppState;
use api::{
    Auth, Room, Spare, SpareInitRequest, SpareInitResponse, SpareListRequest, SpareListResponse,
    SpareQuestionaireRequest, SpareQuestionaireResponse, SpareReturnRequest, SpareReturnResponse,
    SpareTakeRequest, SpareTakeResponse, User, Vacancy,
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
        let submitted_at_s = Utc::now().to_rfc3339();
        for vacancy in req.vacancy {
            let available = matches!(vacancy, Vacancy::Available);
            query("INSERT INTO vacancies (available, submitted_at) VALUES (?, ?)")
                .bind(available)
                .bind(submitted_at_s.clone())
                .execute(&self.database_pool)
                .await
                .unwrap();
        }
        SpareQuestionaireResponse::Success
    }

    async fn spare_take(&self, req: SpareTakeRequest, auth: Auth) -> SpareTakeResponse {
        let res = query(
            "UPDATE spares
                SET assignee = ?
              WHERE id = ?
                AND assignee IS NULL",
        )
        .bind(auth.id as i64)
        .bind(req.id as i64)
        .execute(&self.database_pool)
        .await
        .unwrap();

        if res.rows_affected() == 0 {
            tracing::warn!("spare_take: no unassigned spare with id {}", req.id);
        }
        SpareTakeResponse {}
    }

    async fn spare_return(&self, req: SpareReturnRequest, auth: Auth) -> SpareReturnResponse {
        let res = query(
            "UPDATE spares
                SET assignee = NULL
              WHERE id = ?
                AND assignee = ?",
        )
        .bind(req.id as i64)
        .bind(auth.id as i64)
        .execute(&self.database_pool)
        .await
        .unwrap();

        if res.rows_affected() == 0 {
            tracing::warn!(
                "spare_return: no spare with id {} assigned to user {}",
                req.id,
                auth.id
            );
        }
        SpareReturnResponse {}
    }

    async fn spare_list(&self, req: SpareListRequest, _auth: Auth) -> SpareListResponse {
        let rooms: Vec<Room> = query("SELECT name FROM rooms ORDER BY id")
            .fetch_all(&self.database_pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get("name"))
            .collect();

        let rows = match req {
            SpareListRequest::Schedule => query(
                r#"
                SELECT
                  s.id                     AS id,
                  s.stamp                  AS stamp,
                  s.week                   AS week,
                  s.begin_at               AS begin_at,
                  COALESCE(s.end_at, '')   AS end_at,
                  r.name                   AS room,
                  s.assignee               AS assignee_id,
                  u.username               AS username
                FROM spares s
                JOIN rooms r   ON s.room_id  = r.id
                LEFT JOIN users u ON s.assignee = u.id
                ORDER BY s.id
                "#,
            )
            .fetch_all(&self.database_pool)
            .await
            .unwrap(),

            SpareListRequest::Week(week_str) => query(
                r#"
                SELECT
                  s.id                     AS id,
                  s.stamp                  AS stamp,
                  s.week                   AS week,
                  s.begin_at               AS begin_at,
                  COALESCE(s.end_at, '')   AS end_at,
                  r.name                   AS room,
                  s.assignee               AS assignee_id,
                  u.username               AS username
                FROM spares s
                JOIN rooms r   ON s.room_id  = r.id
                LEFT JOIN users u ON s.assignee = u.id
                WHERE s.week = ?
                ORDER BY s.id
                "#,
            )
            .bind(week_str)
            .fetch_all(&self.database_pool)
            .await
            .unwrap(),
        };

        let spares = rows
            .into_iter()
            .map(|row| Spare {
                id: row.get::<i64, _>("id") as u64,
                stamp: row.get::<i64, _>("stamp") as u64,
                week: row.get::<String, _>("week"),
                begin_time: row.get::<String, _>("begin_at"),
                end_time: row.get::<String, _>("end_at"),
                room: row.get::<String, _>("room"),
                assignee: row.get::<Option<i64>, _>("assignee_id").and_then(|uid| {
                    row.get::<Option<String>, _>("username").map(|u| User {
                        id: uid as u64,
                        username: u,
                    })
                }),
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
            let room_id = (req.rooms.iter().position(|r| r == &spare.room).unwrap() + 1) as i64;
            let assignee = spare.assignee.as_ref().map(|u| u.id as i64);
            query(
                "INSERT INTO spares \
                 (room_id, stamp, begin_at, end_at, week, assignee) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(room_id)
            .bind(spare.stamp as i64)
            .bind(spare.begin_time.clone())
            .bind(Some(spare.end_time.clone()))
            .bind(spare.week.clone())
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
    use sqlx::{query, Row, SqlitePool};

	#[sqlx::test(fixtures("rooms"))]
    async fn test_spare_take(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());

        let now = Utc::now().timestamp();
        let week = now / 604_800;
        query("INSERT INTO spares (room_id, stamp, begin_at, week) VALUES (1, 0, ?, ?)")
            .bind(now)
            .bind(week)
            .execute(&pool)
            .await
            .unwrap();

        let auth = Signer::default().sign(Auth {
            id:        42,
            roles:     vec![Role::user],
            signature: "".into(),
        });
        let _: SpareTakeResponse = app
            .request(APICollection::spare_take(Authed {
                auth: auth.clone(),
                req:  SpareTakeRequest { id: 1 },
            }))
            .await;

        let rec = query("SELECT assignee, end_at FROM spares WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(rec.get::<Option<i64>, _>("assignee"), Some(42));
        assert_eq!(rec.get::<Option<String>, _>("end_at"), None);
    }

    #[sqlx::test(fixtures("rooms"))]
    async fn test_spare_return(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());

        let now = Utc::now().timestamp();
        let week = now / 604_800;
        query("INSERT INTO spares (room_id, stamp, begin_at, week, assignee) VALUES (1, 0, ?, ?, ?)")
            .bind(now)
            .bind(week)
            .bind(7i64)
            .execute(&pool)
            .await
            .unwrap();

        let auth = Signer::default().sign(Auth {
            id:        7,
            roles:     vec![Role::user],
            signature: "".into(),
        });
        let _: SpareReturnResponse = app
            .request(APICollection::spare_return(Authed {
                auth: auth.clone(),
                req:  SpareReturnRequest { id: 1 },
            }))
            .await;

        let rec = query("SELECT assignee FROM spares WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(rec.get::<Option<i64>, _>("assignee"), None);
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

        assert_eq!(list.rooms, vec!["X".to_string(), "Y".to_string()]);
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
              r.name               AS room,
              s.stamp              AS stamp,
              s.begin_at           AS begin_at,
              s.end_at             AS end_at,
              CAST(s.week AS TEXT) AS week,
              s.assignee           AS assignee
            FROM spares s
            JOIN rooms r ON s.room_id = r.id
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.get::<String, _>("room"), "B");
        assert_eq!(row.get::<i64, _>("stamp"), 100);
        assert_eq!(row.get::<String, _>("begin_at"), "15");
        assert_eq!(row.get::<String, _>("end_at"), "25");
        assert_eq!(row.get::<String, _>("week"), "2");
        assert_eq!(row.get::<Option<i64>, _>("assignee"), Some(42));
    }

    #[sqlx::test(fixtures("rooms_xy", "spares_list"))]
    async fn test_spare_list_schedule(pool: SqlitePool) {
        let app = TestApp::new(pool.clone());
        let auth = Signer::default().sign(Auth {
            id: 3,
            roles: vec![Role::user],
            signature: "".into(),
        });

        let list_week: SpareListResponse = app
            .request(APICollection::spare_list(Authed {
                auth: auth.clone(),
                req: SpareListRequest::Week("0".into()),
            }))
            .await;
        let list_sched: SpareListResponse = app
            .request(APICollection::spare_list(Authed {
                auth,
                req: SpareListRequest::Schedule,
            }))
            .await;

        assert_eq!(list_sched.rooms, list_week.rooms);
        assert_eq!(list_sched.spares.len(), list_week.spares.len());
        for (a, b) in list_sched.spares.iter().zip(list_week.spares.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.stamp, b.stamp);
            assert_eq!(a.week, b.week);
            assert_eq!(a.begin_time, b.begin_time);
            assert_eq!(a.end_time, b.end_time);
            assert_eq!(a.room, b.room);
            assert_eq!(a.assignee, b.assignee);
        }
    }
}
