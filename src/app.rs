use api::{APICollection, Error, User, API};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::config::Config;

#[derive(Serialize, Deserialize, Clone)]
pub struct App {
	db: HashMap<String, String>,
	secret: String,
}

impl App {
	pub fn new(config: Config) -> Self {
		todo!();
	}
	pub fn getsecret(&self) -> &str {
		&self.secret
	}
}

impl API for App {
	async fn login(&mut self, _req: api::LoginRequest) -> Result<api::LoginResponse, api::Error> {
		todo!()
	}
	async fn register(&mut self, _req: api::RegisterRequest) -> Result<api::LoginResponse, Error> {
		todo!()
	}
	
	async fn validate(&self, role: api::Role, auth: api::Auth) -> Result<api::Auth, Error> {
		todo!()
	}
	
	async fn spare_return(&mut self, req: api::SpareReturnRequest, auth: api::Auth) -> Result<api::SpareReturnResponse, Error> {
		todo!()
	}
	
	async fn spare_take(&mut self, req: api::SpareTakeRequest, auth: api::Auth) -> Result<api::SpareTakeResponse, Error> {
		todo!()
	}
	
	async fn spare_list(&mut self, req: api::SpareListRequest, auth: api::Auth) -> Result<api::SpareListResponse, Error> {
		todo!()
	}
	
	async fn test_auth_echo(&mut self, req: api::TestAuthEchoRequest, auth: api::Auth) -> Result<api::TestAuthEchoResponse, Error> {
		todo!()
	}
}

