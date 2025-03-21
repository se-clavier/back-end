use api::{APICollection, Error, User, API};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::Deserialize;
use std::fs;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub async fn validate(&self, role: api::Role, auth: api::Auth) -> Result<api::Auth, api::Error> {
	if !auth.roles.contains(&role) {
		return Err(Error {
			code: 483_u16,
			message: "Insufficient role permissions".to_string(),
		});
	}
	#[derive(Debug, Deserialize)]
	struct Claims {
		sub: String,
		exp: usize,
	}
	let secret = fs::read_to_string("/data/secret").map_err(|err| Error {
		code: 500_u16,
		message: format!("Failed to read secret file: {}", err),
	})?;
	let secret_bytes = secret.trim().as_bytes();
	let validation = Validation::new(Algorithm::HS256);
	// 我认为 auth.signature 是一个 JWT 格式了, 不是的话后面再改.
	match decode::<Claims>(&auth.signature, &DecodingKey::from_secret(secret_bytes), &validation) {
		Ok(_token_data) => Ok(auth),
		Err(err) => Err(Error {
			code: 483_u16,
			message: format!("Invalid signature: {}", err),
		}),
	}
}
