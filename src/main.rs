use api::{API, APICollection, Error, User};
use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
struct App {
    db: HashMap<String, String>,
}

impl API for App {
    async fn login(&mut self, _req: api::LoginRequest) -> Result<api::LoginResponse, api::Error> {
        return Err(Error {
            code: 200 as u16,
            message: "token".to_string(),
        });
    }
    async fn login2(&mut self, _req: api::LoginRequest) -> Result<api::LoginResponse2, api::Error> {
        return Ok(api::LoginResponse2 {
            user: User {
                id: 1,
                name: "name".to_string(),
            },
            token: "token".to_string(),
        });
    }
}

async fn handler(
    State(mut app): State<App>,
    Json(body): Json<APICollection>,
) -> Result<Json<impl Serialize>, Json<api::Error>> {
    Ok(Json(app.handle(body).await.map_err(Json)?))
}

#[tokio::main]
async fn main() {
    let app = App { db: HashMap::new() };
    // build our application with a single route
    let router = Router::new().route("/", post(handler)).with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
