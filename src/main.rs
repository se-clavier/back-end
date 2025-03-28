use api::{APICollection, API};
use axum::{extract::State, response::Response, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower_http::cors::CorsLayer;

#[derive(Serialize, Deserialize, Clone)]
struct App {
    db: HashMap<String, String>,
}

impl API for App {}

async fn handler(
    State(mut app): State<App>,
    Json(body): Json<APICollection>,
) -> Result<Json<impl Serialize>, Response> {
    Ok(Json(app.handle(body).await))
}

#[tokio::main]
async fn main() {
    let app = App { db: HashMap::new() };
    // cors allow all
    let cors = CorsLayer::permissive();
    // build our application with a single route
    let router = Router::new()
        .route("/", post(handler))
        .layer(cors)
        .with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
