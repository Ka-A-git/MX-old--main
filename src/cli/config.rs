use serde_derive::Deserialize;
use std::cmp::PartialEq;

pub const CLI_CONFIG_FILE: &str = "conf/cli_config.toml";

#[derive(Deserialize, Debug)]
pub struct CLIConfig {
    pub ip: String,
    pub port: Option<u16>,
    // pub keys: Keys,
}

#[derive(Deserialize, Debug)]
struct Keys {}

impl Default for CLIConfig {
    fn default() -> Self {
        CLIConfig {
            ip: "127.0.0.1".to_string(),
            port: Some(8080),
            // keys: Keys {},
        }
    }
}

impl PartialEq for CLIConfig {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip && self.port == other.port //&& self.keys == other.keys
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::get_config;

    const TEST_CLI_CONFIG_FILE: &str = "test_files/cli_config.toml";

    #[test]
    fn test_parse_file() {
        let cli_config: CLIConfig = get_config(TEST_CLI_CONFIG_FILE).unwrap();

        assert_eq!(cli_config, CLIConfig::default());
    }
}
