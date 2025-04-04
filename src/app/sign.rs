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

    fn gen_sign(&self, auth: &api::Auth) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use api::{Auth, Result, Role};

    #[test]
    fn test_gen_sign_consistency() {
        let secret = "mysecret";
        let signer = Signer::new(secret);
        let auth = Auth {
            id: 1,
            roles: vec![Role::admin],
            signature: String::new(),
        };
        let sign1 = signer.gen_sign(&auth);
        let sign2 = signer.gen_sign(&auth);
        assert_eq!(sign1, sign2, "signature will be same");
    }

    #[test]
    fn test_sign_and_verify() {
        let secret = "mysecret";
        let signer = Signer::new(secret);
        let auth = Auth {
            id: 2,
            roles: vec![Role::admin],
            signature: String::new(),
        };
        let signed_auth = signer.sign(auth);
        assert!(signer.verify(&signed_auth), "signature should be valid");
    }

    #[tokio::test]
    async fn test_validate_authorized() {
        let secret = "mysecret";
        let signer = Signer::new(secret);
        let auth = Auth {
            id: 3,
            roles: vec![Role::admin, Role::user],
            signature: String::new(),
        };
        let signed_auth = signer.sign(auth);
        let expected_signature = signed_auth.signature.clone();
        let result = signer.validate(Role::admin, signed_auth).await;
        match result {
            Result::Ok(valid_auth) => {
                assert_eq!(
                    valid_auth.signature, expected_signature,
                    "auth after validation should have the same signature"
                );
            }
            _ => panic!("Expected authorized, but got unauthorized"),
        }
    }

    #[tokio::test]
    async fn test_validate_unauthorized() {
        let secret = "mysecret";
        let signer = Signer::new(secret);
        let auth = Auth {
            id: 4,
            roles: vec![Role::user],
            signature: String::new(),
        };
        let signed_auth = signer.sign(auth);
        let result = signer.validate(Role::admin, signed_auth).await;
        match result {
            Result::Unauthorized => (),
            _ => panic!("Expected unauthorized, but got authorized"),
        }
    }
}
