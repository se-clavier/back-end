use api::{Auth, Result};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct Signer {
    mac: HmacSha256,
}

impl Signer {
    pub fn new(secret: &str) -> Self {
        let mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("Secret key error");
        Self { mac }
    }

    pub fn gen_sign(&self, auth: &api::Auth) -> String {
        let mut mac = self.mac.clone();

        let data_json = json!({
            "id": auth.id,
            "roles": auth.roles,
        });
        let data = serde_json::to_string(&data_json).expect("JSON serialization error");
        mac.update(data.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    pub fn sign(&self, mut auth: api::Auth) -> api::Auth {
        auth.signature = self.gen_sign(&auth);
        auth
    }

    pub async fn validate(&self, role: api::Role, auth: api::Auth) -> Result<Auth> {
        let expected_sign = self.gen_sign(&auth);
        if expected_sign != auth.signature {
            return api::Result::Unauthorized;
        }
        if auth.roles.contains(&role) {
            api::Result::Ok(auth)
        } else {
            api::Result::Unauthorized
        }
    }
	pub fn verify(&self, auth: &api::Auth) -> bool {
        let expect_sign = self.gen_sign(auth);
        expect_sign == auth.signature
    }
	
}
