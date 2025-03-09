use api::{APICollection, Error, User, API};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
struct App {
    db: HashMap<String, String>,
}

impl API for App {
    async fn login(&mut self, _req: api::LoginRequest) -> Result<api::LoginResponse, api::Error> {
        Err(Error {
            code: 400_u16,
            message: "bad request".to_string(),
        })
    }
    async fn register(&mut self, _req: api::RegisterRequest) -> Result<api::LoginResponse, Error> {
        Ok(api::LoginResponse {
            user: User {
                id: 1,
                name: "rnoob".to_string(),
                roles: vec![api::Role::user],
            },
            token: "token".to_string(),
        })
    }
}

async fn handler(
    State(mut app): State<App>,
    Json(body): Json<APICollection>,
) -> Result<Json<impl Serialize>, Response> {
    Ok(Json(app.handle(body).await.map_err(|e| {
        (StatusCode::from_u16(e.code).unwrap(), Json(e)).into_response()
    })?))
}

#[tokio::main]
async fn main() {
    let app = App { db: HashMap::new() };
    // build our application with a single route
    let router = Router::new().route("/", post(handler)).with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
