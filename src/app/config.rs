use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub secret: String,
    pub salt: String,
    // TODO: Add configuration options
}

impl Default for Config {
    fn default() -> Self {
        Self {
            secret: String::from(super::DEFAULT_SECRET),
            salt: String::from(super::DEFAULT_SALT),
        }
    }
}