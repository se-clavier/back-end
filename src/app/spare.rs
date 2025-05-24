use super::{algorithm::max_flow, parse_time_delta, AppState, CheckinStatus};
use api::{
    Auth, Room, Spare, SpareAutoAssignRequest, SpareAutoAssignResponse, SpareInitRequest,
    SpareInitResponse, SpareListRequest, SpareListResponse, SpareQuestionaireRequest,
    SpareQuestionaireResponse, SpareReturnRequest, SpareReturnResponse, SpareSetAssigneeRequest,
    SpareSetAssigneeResponse, SpareTakeRequest, SpareTakeResponse, User, Vacancy,
};

use sqlx::{query, query_as, types::Json, Executor, QueryBuilder, Row};

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
    async fn spare_set_assignee(
        &self,
        req: SpareSetAssigneeRequest,
        auth: Auth,
    ) -> SpareSetAssigneeResponse;
    async fn spare_trigger_assign(
        &self,
        req: SpareAutoAssignRequest,
        auth: Auth,
    ) -> SpareAutoAssignResponse;
}

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

    async fn spare_list(&self, req: SpareListRequest, auth: Auth) -> SpareListResponse {
        let mut tx = self.database_pool.begin().await.unwrap();

        let rooms: Vec<Room> = query("SELECT name FROM rooms ORDER BY id")
            .fetch_all(&mut *tx)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get("name"))
            .collect();

        #[derive(sqlx::FromRow)]
        struct SpareRow {
            id: u64,
            stamp: u64,
            week: String,
            begin_at: String,
            end_at: String,
            room: String,
            assignee_id: Option<u64>,
            username: Option<String>,
        }
        let spares = match req {
            SpareListRequest::Schedule => query_as(
                r#"
                    SELECT
                      s.id                     AS id,
                      s.stamp                  AS stamp,
                      s.week                   AS week,
                      s.begin_at               AS begin_at,
                      s.end_at                 AS end_at,
                      r.name                   AS room,
                      a.user_id                AS assignee_id,
                      u.username               AS username
                    FROM spares s
                    JOIN rooms r   ON s.room_id  = r.id
                    LEFT JOIN availables a ON s.stamp = a.stamp AND a.user_id = ?
                    LEFT JOIN users u ON a.user_id = u.id
                    WHERE s.week = ?
                    ORDER BY s.id
                    "#,
            )
            .bind(auth.id as i64)
            .bind("schedule")
            .fetch_all(&self.database_pool)
            .await
            .unwrap(),
            SpareListRequest::Week(week_str) => query_as(
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
            .bind(&week_str)
            .fetch_all(&self.database_pool)
            .await
            .unwrap(),
        }
        .into_iter()
        .map(|row: SpareRow| Spare {
            id: row.id,
            stamp: row.stamp,
            week: row.week,
            begin_time: row.begin_at,
            end_time: row.end_at,
            room: row.room,
            assignee: row
                .assignee_id
                .and_then(|id| row.username.map(|username| User { id, username })),
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
                    .push_bind(Json(CheckinStatus::None));
            },
        );
        let spares_query = spares_qb.build();
        tx.execute(spares_query).await.unwrap();

        tx.commit().await.unwrap();

        SpareInitResponse::Success
    }

    async fn spare_set_assignee(
        &self,
        req: SpareSetAssigneeRequest,
        _auth: Auth,
    ) -> SpareSetAssigneeResponse {
        let mut tx = self.database_pool.begin().await.unwrap();

        query(
            "UPDATE spares
                SET assignee = ?
              WHERE id = ?",
        )
        .bind(req.assignee.map(|u| u.id as i64))
        .bind(req.id as i64)
        .execute(&mut *tx)
        .await
        .unwrap();

        tx.commit().await.unwrap();

        SpareSetAssigneeResponse::Success
    }

    #[allow(unused)]
    async fn spare_trigger_assign(
        &self,
        req: SpareAutoAssignRequest,
        auth: Auth,
    ) -> SpareAutoAssignResponse {
        let mut tx = self.database_pool.begin().await.unwrap();
        let users: Vec<_> = query_as(
            "
            SELECT user_id, json_group_array(stamp) FROM availables
                GROUP BY user_id
            ",
        )
        .fetch_all(&mut *tx)
        .await
        .unwrap()
        .into_iter()
        .map(|(user_id, stamps): (i64, Json<Vec<usize>>)| (user_id, stamps.0))
        .collect();

        let spares = query_as(
            "
            SELECT
                stamp,
                begin_at
                FROM spares
                WHERE week = 'schedule'
                ORDER BY stamp
            ",
        )
        .fetch_all(&mut *tx)
        .await
        .unwrap()
        .into_iter()
        .map(|(stamp, begin_at): (i64, String)| parse_time_delta(begin_at).num_days() as usize)
        .collect();

        let assignees = max_flow(users, spares);
        for (stamp, assignee) in assignees.into_iter().enumerate() {
            let mut qb = QueryBuilder::new(
                "UPDATE spares
                    SET assignee = ",
            );
            qb.push_bind(assignee);
            qb.push(" WHERE (stamp, week) IN ");
            qb.push_tuples(req.weeks.iter(), |mut b, week| {
                b.push_bind(stamp as i64);
                b.push_bind(week);
            });
            tracing::info!("sql: {}", qb.sql());
            let query = qb.build();
            query.execute(&mut *tx).await.unwrap();
        }
        tx.commit().await.unwrap();

        SpareAutoAssignResponse::Success
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

    #[sqlx::test(fixtures("users", "spares", "availables"))]
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
                    id: 3,
                    stamp: 0,
                    week: String::from("schedule"),
                    begin_time: String::from("P0Y0M0DT8H0M0S"),
                    end_time: String::from("P0Y0M0DT10H0M0S"),
                    room: String::from("room1"),
                    assignee: Some(User {
                        id: 1,
                        username: String::from("testuser"),
                    })
                },
                Spare {
                    id: 5,
                    stamp: 1,
                    week: String::from("schedule"),
                    begin_time: String::from("P0Y0M1DT8H0M0S"),
                    end_time: String::from("P0Y0M1DT10H0M0S"),
                    room: String::from("room1"),
                    assignee: Some(User {
                        id: 1,
                        username: String::from("testuser"),
                    })
                },
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
                assignee: None,
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

    #[sqlx::test(fixtures("users", "spares"))]
    async fn test_spare_set_assignee(pool: SqlitePool) {
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
            .spare_set_assignee(
                SpareSetAssigneeRequest {
                    id: 2,
                    assignee: None,
                },
                auth,
            )
            .await;

        assert_eq!(res, SpareSetAssigneeResponse::Success);
    }
    #[sqlx::test(fixtures("users", "spares", "availables"))]
    async fn test_spare_trigger_assign(pool: SqlitePool) {
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
            .spare_trigger_assign(
                SpareAutoAssignRequest {
                    weeks: vec![String::from("2000-W21")],
                },
                auth.clone(),
            )
            .await;

        assert_eq!(res, SpareAutoAssignResponse::Success);

        let list = app
            .spare_list(
                SpareListRequest::Week(String::from("2000-W21")),
                auth.clone(),
            )
            .await;

        assert_eq!(list.rooms, vec![String::from("room1")]);
        assert_eq!(
            list.spares,
            vec![
                Spare {
                    id: 6,
                    stamp: 0,
                    week: String::from("2000-W21"),
                    begin_time: String::from("P0Y0M0DT8H0M0S"),
                    end_time: String::from("P0Y0M0DT10H0M0S"),
                    room: String::from("room1"),
                    assignee: Some(User {
                        id: 2,
                        username: String::from("testadmin"),
                    }),
                },
                Spare {
                    id: 7,
                    stamp: 1,
                    week: String::from("2000-W21"),
                    begin_time: String::from("P0Y0M1DT8H0M0S"),
                    end_time: String::from("P0Y0M1DT10H0M0S"),
                    room: String::from("room1"),
                    assignee: Some(User {
                        id: 1,
                        username: String::from("testuser"),
                    }),
                },
            ]
        );
    }
}
