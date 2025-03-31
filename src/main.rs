mod app;

use api::{APICollection, API};
use app::AppState;
use axum::{extract::State, response::Response, routing::post, Json, Router};
use serde::Serialize;
use tower_http::cors::CorsLayer;

const DATABASE_URL: &str = "sqlite://db/sqlite.db";

async fn handler(
    State(mut app): State<AppState>,
    Json(body): Json<APICollection>,
) -> Result<Json<impl Serialize>, Response> {
    Ok(Json(app.handle(body).await))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    // Create AppState
    let app = AppState::new(DATABASE_URL).await?;
    // cors allow all
    let cors = CorsLayer::permissive();
    // build our application with a single route
    let router = Router::new()
        .route("/", post(handler))
        .layer(cors)
        .with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await?;
    axum::serve(listener, router).await?;
    Ok(())
}
