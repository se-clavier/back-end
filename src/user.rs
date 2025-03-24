use api::{APICollection, Error, User, API};
use axum::routing::get;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::app::App;
use hex::encode;
use serde_json::json;

type HmacSha256 = Hmac<Sha256>;

impl App {
	/// 生成签名
	fn generate_signature(&self, id: &u64, roles: &Vec<api::Role>) -> Result<String, api::Error> {
		let mut mac = HmacSha256::new_from_slice(self.getsecret().as_bytes())
			.map_err(|_| api::Error {
				code: 500_u16,
				message: "Secret key error".to_string(),
			})?;

		let data_json = json!({
			"id": id,
			"roles": roles,
		});
		let data = serde_json::to_string(&data_json).map_err(|_| api::Error {
			code: 500_u16,
			message: "JSON serialization error".to_string(),
		})?;
		mac.update(data.as_bytes());
		let result = mac.finalize();
		Ok(hex::encode(result.into_bytes()))
	}

	/// 验证 Auth 对象
	pub async fn validate(&self, role: api::Role, auth: api::Auth) -> Result<api::Auth, api::Error> {
		let expected_signature = self.generate_signature(&auth.id, &auth.roles)?;
		
		if expected_signature != auth.signature {
			return Err(api::Error {
				code: 483_u16,
				message: "Invalid signature".to_string(),
			});
		}

		if auth.roles.contains(&role) {
			Ok(auth)
		} else {
			Err(api::Error {
				code: 483_u16,
				message: "roles not match".to_string(),
			})
		}
	}
}
