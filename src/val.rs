use crate::app::App;
use api::{APICollection, Auth, Result, User, API};
use axum::routing::get;
use hex::encode;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

impl App {
    /// 生成签名，返回一个字符串
    fn generate_signature(&self, id: &u64, roles: &Vec<api::Role>) -> String {
        let mut mac =
            HmacSha256::new_from_slice(self.get_secret().as_bytes()).expect("Secret key error");
        let data_json = json!({
            "id": id,
            "roles": roles,
        });
        let data = serde_json::to_string(&data_json).expect("JSON serialization error");
        mac.update(data.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// 验证 Auth 对象，返回 api::Result<Auth>
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
