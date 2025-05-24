use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;

#[derive(Debug, Clone)]
pub struct Hasher {
    argon2: Argon2<'static>,
}

impl Hasher {
    pub fn new() -> Self {
        Self {
            argon2: Argon2::default(),
        }
    }

    pub fn hash(&self, password: &str) -> String {
        self.argon2
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
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
        Self::new()
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
