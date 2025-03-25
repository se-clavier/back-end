use api::{APICollection, Error, API};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower_http::cors::CorsLayer;

#[derive(Serialize, Deserialize, Clone)]
struct App {
    db: HashMap<String, String>,
}

impl API for App {
    async fn login(&mut self, _req: api::LoginRequest) -> Result<api::LoginResponse, api::Error> {
        todo!()
    }
    async fn register(&mut self, _req: api::RegisterRequest) -> Result<api::LoginResponse, Error> {
        todo!()
    }

    async fn validate(&self, _role: api::Role, _auth: api::Auth) -> Result<api::Auth, Error> {
        todo!()
    }

    async fn spare_return(
        &mut self,
        _req: api::SpareReturnRequest,
        _auth: api::Auth,
    ) -> Result<api::SpareReturnResponse, Error> {
        todo!()
    }

    async fn spare_take(
        &mut self,
        _req: api::SpareTakeRequest,
        _auth: api::Auth,
    ) -> Result<api::SpareTakeResponse, Error> {
        todo!()
    }

    async fn spare_list(
        &mut self,
        _req: api::SpareListRequest,
        _auth: api::Auth,
    ) -> Result<api::SpareListResponse, Error> {
        todo!()
    }

    async fn test_auth_echo(
        &mut self,
        _req: api::TestAuthEchoRequest,
        _auth: api::Auth,
    ) -> Result<api::TestAuthEchoResponse, Error> {
        todo!()
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
    // cors allow all
    let cors = CorsLayer::permissive();
    // build our application with a single route
    let router = Router::new()
        .route("/", post(handler))
        .layer(cors)
        .with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
