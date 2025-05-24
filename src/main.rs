mod app;
mod config;

use app::{app, connect_pool};
use config::Config;

const DATABASE_URL: &str = "sqlite://db/sqlite.db";
const CONFIG_PATH: &str = "cfg/config.json";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    tracing::info!("Starting application");
    let app = app(
        connect_pool(DATABASE_URL).await,
        Config::parse_cfg(CONFIG_PATH),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await.unwrap();
    tracing::info!("Listening on {:?}", listener);

    tracing::info!("Starting server");
    axum::serve(listener, app).await.unwrap();
}
