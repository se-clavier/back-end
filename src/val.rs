use crate::app::App;
use api::{APICollection, Auth, Result, User, API};
use axum::routing::get;
use hex::encode;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct SignatureGenerator {
    mac: HmacSha256,
}

impl SignatureGenerator {
    pub fn new(secret: &str) -> Self {
        let mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("Secret key error");
        Self { mac }
    }

    /// 生成签名：通过克隆内部的 mac 来计算签名
    pub fn generate_signature(&self, id: &u64, roles: &Vec<api::Role>) -> String {
        let mut mac_clone = self.mac.clone();
        let data_json = json!({
            "id": id,
            "roles": roles,
        });
        let data = serde_json::to_string(&data_json).expect("JSON serialization error");
        mac_clone.update(data.as_bytes());
        let result = mac_clone.finalize();
        hex::encode(result.into_bytes())
    }
}

impl App {
    fn generate_signature(&self, id: &u64, roles: &Vec<api::Role>) -> String {
        self.signature_generator.generate_signature(id, roles)
    }

    pub async fn validate(&self, role: api::Role, auth: api::Auth) -> Result<Auth> {
        let expected_signature = self.generate_signature(&auth.id, &auth.roles);

        if expected_signature != auth.signature {
            return api::Result::Unauthorized;
        }

        if auth.roles.contains(&role) {
            api::Result::Ok(auth)
        } else {
            api::Result::Unauthorized
        }
    }
}
