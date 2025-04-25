use super::AppState;
use api::*;

use sqlx::{query, Row, Executor, QueryBuilder};

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
        _req: SpareQuestionaireRequest,
        _auth: Auth,
    ) -> SpareQuestionaireResponse {
        todo!();
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
            tracing::error!("spare_take: no unassigned spare with id {}", req.id);
            panic!("spare_take: no unassigned spare with id {}", req.id);
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
            tracing::error!(
                "spare_return: no spare with id {} assigned to user {}",
                req.id,
                auth.id
            );
            panic!("spare_take: no unassigned spare with id {}", req.id);
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
                  s.end_at                 AS end_at,
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
                week: row.get("week"),
                begin_time: row.get("begin_at"),
                end_time: row.get("end_at"),
                room: row.get("room"),
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
		let mut tx = self.database_pool.begin().await.unwrap();
	
		tx.execute(query("DELETE FROM spares")).await.unwrap();
		tx.execute(query("DELETE FROM sqlite_sequence WHERE name='spares'")).await.unwrap();
		tx.execute(query("DELETE FROM rooms")).await.unwrap();
		tx.execute(query("DELETE FROM sqlite_sequence WHERE name='rooms'")).await.unwrap();
	
		let mut rooms_qb = QueryBuilder::new("INSERT INTO rooms (name)");
		rooms_qb.push_values(req.rooms.iter(), |mut b, room| {
			b.push_bind(room);
		});
		let rooms_query = rooms_qb.build();
		tx.execute(rooms_query).await.unwrap();
	
		let mut spares_qb = QueryBuilder::new(
			"INSERT INTO spares (room_id, stamp, begin_at, end_at, week, assignee)"
		);
		spares_qb.push_values(
			req.spares.iter().flat_map(|spare| {
				let room_id = (req.rooms.iter().position(|r| r == &spare.room).unwrap() + 1) as i64;
				let assignee = spare.assignee.as_ref().map(|u| u.id as i64);
				req.weeks.iter().map(move |week| {
					(
						room_id,
						spare.stamp as i64,
						spare.begin_time.as_str(),
						spare.end_time.as_str(),
						week.as_str(),
						assignee,
					)
				})
			}),
			|mut b, (room_id, stamp, begin_time, end_time, week, assignee)| {
				b.push_bind(room_id)
				 .push_bind(stamp)
				 .push_bind(begin_time)
				 .push_bind(end_time)
				 .push_bind(week)
				 .push_bind(assignee);
			},
		);
		let spares_query = spares_qb.build();
		tx.execute(spares_query).await.unwrap();
	
		tx.commit().await.unwrap();
	
		SpareInitResponse::Success
	}
	
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::test::TestApp;

    use sqlx::SqlitePool;

    #[sqlx::test(fixtures("users", "spares"))]
    async fn test_spare_take(pool: SqlitePool) {
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

        let _ = app.spare_take(SpareTakeRequest { id: 1 }, auth).await;
    }

    #[sqlx::test(fixtures("users", "spares"))]
    async fn test_spare_return(pool: SqlitePool) {
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

        let _ = app.spare_return(SpareReturnRequest { id: 2 }, auth).await;
    }

    #[sqlx::test(fixtures("users", "spares"))]
    async fn test_spare_list_week(pool: SqlitePool) {
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

        let list = app
            .spare_list(SpareListRequest::Week(String::from("week1")), auth)
            .await;

        assert_eq!(list.rooms, vec![String::from("room1")]);
        assert_eq!(
            list.spares,
            vec![Spare {
                id: 1,
                stamp: 0,
                week: String::from("week1"),
                begin_time: String::from("begin1"),
                end_time: String::from("end1"),
                room: String::from("room1"),
                assignee: None
            }]
        );
    }

    #[sqlx::test(fixtures("users", "spares"))]
    async fn test_spare_list_schedule(pool: SqlitePool) {
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

        let list = app.spare_list(SpareListRequest::Schedule, auth).await;

        assert_eq!(list.rooms, vec![String::from("room1")]);
        assert_eq!(
            list.spares,
            vec![
                Spare {
                    id: 1,
                    stamp: 0,
                    week: String::from("week1"),
                    begin_time: String::from("begin1"),
                    end_time: String::from("end1"),
                    room: String::from("room1"),
                    assignee: None
                },
                Spare {
                    id: 2,
                    stamp: 0,
                    week: String::from("week2"),
                    begin_time: String::from("begin2"),
                    end_time: String::from("end2"),
                    room: String::from("room1"),
                    assignee: Some(User {
                        id: 1,
                        username: String::from("testuser")
                    })
                }
            ]
        );
    }
    #[sqlx::test(fixtures("users", "spares"))]
    async fn test_spare_init(pool: SqlitePool) {
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
        let rooms = vec![String::from("test_room1")];
        let spares = vec![
            Spare {
                id: 1,
                stamp: 0,
                week: String::from("test_week1"),
                begin_time: String::from("test_begin1"),
                end_time: String::from("test_end1"),
                room: String::from("test_room1"),
                assignee: None,
            },
            Spare {
                id: 2,
                stamp: 0,
                week: String::from("test_week1"),
                begin_time: String::from("test_begin2"),
                end_time: String::from("test_end2"),
                room: String::from("test_room1"),
                assignee: Some(User {
                    id: 1,
                    username: String::from("testuser"),
                }),
            },
        ];

        assert_eq!(
            app.spare_init(
                SpareInitRequest {
                    weeks: vec![String::from("test_week1")],
                    rooms: rooms.clone(),
                    spares: spares.clone()
                },
                auth.clone()
            )
            .await,
            SpareInitResponse::Success
        );

        assert_eq!(
            app.spare_list(SpareListRequest::Schedule, auth).await,
            SpareListResponse { rooms, spares }
        )
    }
}
