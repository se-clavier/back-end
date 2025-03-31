mod app;

const DATABASE_URL: &str = "sqlite://db/sqlite.db";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await.unwrap();
    tracing::info!("Listening on {:?}", listener);
    let app = app::app(DATABASE_URL).await;
    axum::serve(listener, app).await.unwrap();
}
