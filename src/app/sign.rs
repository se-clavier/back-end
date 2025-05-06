use api::{Auth, Result};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct Signer {
    mac: HmacSha256,
}

impl Default for Signer {
    fn default() -> Self {
        Self::new(super::DEFAULT_SECRET)
    }
}

impl Signer {
    pub fn new(secret: &str) -> Self {
        let mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("Secret key error");
        Self { mac }
    }

    fn gen_sign(&self, auth: &api::Auth) -> String {
        let mut mac = self.mac.clone();
        let auth = Auth {
            signature: String::new(),
            ..auth.clone()
        };
        let data = serde_json::to_string(&auth).expect("JSON serialization error");
        mac.update(data.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    pub fn sign(&self, mut auth: api::Auth) -> api::Auth {
        auth.signature = self.gen_sign(&auth);
        auth
    }

    pub fn validate(&self, role: api::Role, auth: api::Auth) -> Result<Auth> {
        if self.gen_sign(&auth) != auth.signature {
            return api::Result::Unauthorized;
        }
        let expire: DateTime<Utc> = auth.expire.parse().unwrap();
        if Utc::now() > expire {
            return api::Result::Unauthorized;
        }
        if auth.roles.contains(&role) {
            api::Result::Ok(auth)
        } else {
            api::Result::Unauthorized
        }
    }
}

#[cfg(test)]
pub mod test {

    use super::*;
    use api::{Auth, Result, Role};
    use chrono::TimeDelta;

    #[test]
    fn test_gen_sign_consistency() {
        let signer = Signer::default();
        let auth = Auth {
            id: 1,
            roles: vec![Role::admin],
            expire: (Utc::now() + TimeDelta::days(1)).to_rfc3339(),
            signature: String::new(),
        };
        let sign1 = signer.gen_sign(&auth);
        let sign2 = signer.gen_sign(&auth);
        assert_eq!(sign1, sign2, "signature will be same");
    }

    #[test]
    fn test_validate_authorized() {
        let signer = Signer::default();
        let auth = Auth {
            id: 3,
            roles: vec![Role::admin, Role::user],
            expire: (Utc::now() + TimeDelta::days(1)).to_rfc3339(),
            signature: String::new(),
        };
        let signed_auth = signer.sign(auth);
        let expected_signature = signed_auth.signature.clone();
        let result = signer.validate(Role::admin, signed_auth);
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

    #[test]
    fn test_validate_unauthorized() {
        let signer = Signer::default();
        let auth = Auth {
            id: 4,
            roles: vec![Role::user],
            expire: (Utc::now() + TimeDelta::days(1)).to_rfc3339(),
            signature: String::new(),
        };
        let signed_auth = signer.sign(auth);
        let result = signer.validate(Role::admin, signed_auth);
        assert_eq!(
            result,
            Result::Unauthorized,
            "Expected unauthorized, but got authorized"
        );
    }
}
