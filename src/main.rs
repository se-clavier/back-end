use api::{APICollection, Error, User, API};
use axum::{
	extract::State,http::StatusCode,
	response::{IntoResponse, Response},
	routing::post, Json, Router,
};
use backend::config::Config;
use backend::app::App;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
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
	let config = Config::parse();
	let app = App::new(config);
	let cors = CorsLayer::permissive();
	let router = Router::new().route("/", post(handler)).layer(cors).with_state(app);
	let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
	axum::serve(listener, router).await.unwrap();
}
