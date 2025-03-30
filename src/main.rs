use api::{APICollection, API};
use axum::{extract::State, response::Response, routing::post, Json, Router};
use serde::Serialize;
use sqlx::{
    migrate::MigrateDatabase, Sqlite, SqlitePool,
};
use tower_http::cors::CorsLayer;
const DATABASE_URL: &str = "sqlite://sqlite.db";
#[derive(Clone)]
struct App {
    pool: SqlitePool,
}

impl API for App {}

async fn handler(
    State(mut app): State<App>,
    Json(body): Json<APICollection>,
) -> Result<Json<impl Serialize>, Response> {
    let _conn = app.pool.acquire().await;
    Ok(Json(app.handle(body).await))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    // Create a new SQLite database if it doesn't exist
    if !Sqlite::database_exists(DATABASE_URL).await? {
        Sqlite::create_database(DATABASE_URL).await?;
    }

    // Create a new SQLite connection pool
    let pool = SqlitePool::connect(DATABASE_URL).await?;
    
    // Run migrations
    sqlx::migrate!().run(&pool).await?;

    let app = App { pool };
    // cors allow all
    let cors = CorsLayer::permissive();
    // build our application with a single route
    let router = Router::new()
        .route("/", post(handler))
        .layer(cors)
        .with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await.unwrap();
    axum::serve(listener, router).await.unwrap();
    Ok(())
}