use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub secret: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            secret: String::from("mysecret"),
        }
    }
}

impl Config {
    pub fn parse_cfg(path: &str) -> Self {
        serde_json::from_str(std::fs::read_to_string(path).unwrap().as_str()).unwrap()
    }
}
