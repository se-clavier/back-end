use app::{app, connect_pool};
use tracing_subscriber::util::SubscriberInitExt;

mod app;

const DATABASE_URL: &str = "sqlite://db/sqlite.db";

#[tokio::main]
async fn main() {
    let app = app(
        connect_pool(DATABASE_URL).await,
        Default::default(),
        tracing_subscriber::fmt().set_default(),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await.unwrap();
    tracing::info!("Listening on {:?}", listener);

    tracing::info!("Starting server");
    axum::serve(listener, app).await.unwrap();
}
