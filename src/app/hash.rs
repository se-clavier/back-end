use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
#[derive(Debug, Clone)]
pub struct Hasher {
    salt: SaltString,
    argon2: Argon2<'static>,
}

impl Hasher {
    pub fn new(salt: &str) -> Self {
        Self {
            salt: SaltString::from_b64(salt).unwrap(),
            argon2: Argon2::default(),
        }
    }

    pub fn hash(&self, password: &str) -> String {
        self.argon2
            .hash_password(password.as_bytes(), &self.salt)
            .unwrap()
            .to_string()
    }

    pub fn verify(&self, password: &str, hash: &str) -> bool {
        self.argon2
            .verify_password(password.as_bytes(), &PasswordHash::new(hash).unwrap())
            .is_ok()
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new(super::DEFAULT_SALT)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_hash() {
        let hasher = Hasher::default();
        let password = "password123";
        let hash = hasher.hash(password);
        println!("Hash: {}", hash);
        assert!(hasher.verify(password, &hash));
    }
}
