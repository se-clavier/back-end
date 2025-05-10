use super::{AppState, CheckinStatus};
use api::{
    Auth, Room, Spare, SpareInitRequest, SpareInitResponse, SpareListRequest, SpareListResponse,
    SpareQuestionaireRequest, SpareQuestionaireResponse, SpareReturnRequest, SpareReturnResponse,
    SpareTakeRequest, SpareTakeResponse, User, Vacancy,
};

use sqlx::{query, Executor, QueryBuilder, Row};

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
        auth: Auth,
    ) -> SpareQuestionaireResponse {
        let mut tx = self.database_pool.begin().await.unwrap();

        query(
            "DELETE FROM availables
                WHERE user_id = ?",
        )
        .bind(auth.id as i64)
        .execute(&mut *tx)
        .await
        .unwrap();

        QueryBuilder::new("INSERT INTO availables (user_id, stamp)")
            .push_values(
                req.vacancy
                    .into_iter()
                    .enumerate()
                    .filter_map(|(stamp, vacancy)| match vacancy {
                        Vacancy::Available => Some(stamp),
                        Vacancy::Unavailable => None,
                    }),
                |mut b, stamp| {
                    b.push_bind(auth.id as i64);
                    b.push_bind(stamp as i64);
                },
            )
            .build()
            .execute(&mut *tx)
            .await
            .unwrap();

        tx.commit().await.unwrap();

        SpareQuestionaireResponse::Success
    }

    async fn spare_take(&self, req: SpareTakeRequest, auth: Auth) -> SpareTakeResponse {
        let mut tx = self.database_pool.begin().await.unwrap();

        let res = query(
            "UPDATE spares
                SET assignee = ?
              WHERE id = ?
                AND assignee IS NULL",
        )
        .bind(auth.id as i64)
        .bind(req.id as i64)
        .execute(&mut *tx)
        .await
        .unwrap();

        if res.rows_affected() == 0 {
            tracing::error!("spare_take: no unassigned spare with id {}", req.id);
            panic!("spare_take: no unassigned spare with id {}", req.id);
        }

        tx.commit().await.unwrap();

        SpareTakeResponse {}
    }

    async fn spare_return(&self, req: SpareReturnRequest, auth: Auth) -> SpareReturnResponse {
        let mut tx = self.database_pool.begin().await.unwrap();

        let res = query(
            "UPDATE spares
                SET assignee = NULL
              WHERE id = ?
                AND assignee = ?",
        )
        .bind(req.id as i64)
        .bind(auth.id as i64)
        .execute(&mut *tx)
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

        tx.commit().await.unwrap();

        SpareReturnResponse {}
    }

    async fn spare_list(&self, req: SpareListRequest, _auth: Auth) -> SpareListResponse {
        let mut tx = self.database_pool.begin().await.unwrap();

        let rooms: Vec<Room> = query("SELECT name FROM rooms ORDER BY id")
            .fetch_all(&mut *tx)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get("name"))
            .collect();

        let week = match req {
            SpareListRequest::Schedule => String::from("schedule"),
            SpareListRequest::Week(week_str) => week_str,
        };

        let rows = query(
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
                WHERE s.week = ?
                ORDER BY s.id
                "#,
        )
        .bind(&week)
        .fetch_all(&self.database_pool)
        .await
        .unwrap();

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

        tx.commit().await.unwrap();

        SpareListResponse { rooms, spares }
    }

    async fn spare_init(&self, req: SpareInitRequest, _auth: Auth) -> SpareInitResponse {
        let mut tx = self.database_pool.begin().await.unwrap();

        tx.execute(query("DELETE FROM spares")).await.unwrap();
        tx.execute(query("DELETE FROM sqlite_sequence WHERE name='spares'"))
            .await
            .unwrap();
        tx.execute(query("DELETE FROM rooms")).await.unwrap();
        tx.execute(query("DELETE FROM sqlite_sequence WHERE name='rooms'"))
            .await
            .unwrap();
        tx.execute(query("DELETE FROM availables")).await.unwrap();
        tx.execute(query("DELETE FROM sqlite_sequence WHERE name='availables'"))
            .await
            .unwrap();

        let mut rooms_qb = QueryBuilder::new("INSERT INTO rooms (name)");
        rooms_qb.push_values(req.rooms.iter(), |mut b, room| {
            b.push_bind(room);
        });
        let rooms_query = rooms_qb.build();
        tx.execute(rooms_query).await.unwrap();

        let mut spares_qb = QueryBuilder::new(
            "INSERT INTO spares (room_id, stamp, begin_at, end_at, week, assignee, status)",
        );

        spares_qb.push_values(
            req.spares.iter().flat_map(|spare| {
                let room_id = (req.rooms.iter().position(|r| r == &spare.room).unwrap() + 1) as i64;
                let assignee = spare.assignee.as_ref().map(|u| u.id as i64);
                req.weeks
                    .iter()
                    .map(move |week| {
                        (
                            room_id,
                            spare.stamp as i64,
                            spare.begin_time.as_str(),
                            spare.end_time.as_str(),
                            week.as_str(),
                            assignee.clone(),
                        )
                    })
                    .chain(
                        Some((
                            room_id,
                            spare.stamp as i64,
                            spare.begin_time.as_str(),
                            spare.end_time.as_str(),
                            "schedule",
                            assignee.clone(),
                        ))
                        .into_iter(),
                    )
            }),
            |mut b, (room_id, stamp, begin_time, end_time, week, assignee)| {
                b.push_bind(room_id)
                    .push_bind(stamp)
                    .push_bind(begin_time)
                    .push_bind(end_time)
                    .push_bind(week)
                    .push_bind(assignee)
                    .push_bind(CheckinStatus::None);
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

    use api::{LoginRequest, LoginResponse, RevAPI};
    use sqlx::SqlitePool;

    #[sqlx::test(fixtures("users"))]
    async fn test_spare_questionaire(pool: SqlitePool) {
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

        let _ = app
            .spare_questionaire(
                SpareQuestionaireRequest {
                    vacancy: vec![Vacancy::Available, Vacancy::Unavailable, Vacancy::Available],
                },
                auth,
            )
            .await;
    }

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
            .spare_list(SpareListRequest::Week(String::from("2000-W18")), auth)
            .await;

        assert_eq!(list.rooms, vec![String::from("room1")]);
        assert_eq!(
            list.spares,
            vec![Spare {
                id: 1,
                stamp: 0,
                week: String::from("2000-W18"),
                begin_time: String::from("P0Y0M0DT8H0M0S"),
                end_time: String::from("P0Y0M0DT10H0M0S"),
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
            vec![Spare {
                id: 3,
                stamp: 0,
                week: String::from("schedule"),
                begin_time: String::from("P0Y0M0DT8H0M0S"),
                end_time: String::from("P0Y0M0DT10H0M0S"),
                room: String::from("room1"),
                assignee: None
            },]
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
                id: 2,
                stamp: 0,
                week: String::from("schedule"),
                begin_time: String::from("test_begin1"),
                end_time: String::from("test_end1"),
                room: String::from("test_room1"),
                assignee: None,
            },
            Spare {
                id: 4,
                stamp: 0,
                week: String::from("schedule"),
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
