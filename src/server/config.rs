use serde_derive::Deserialize;

pub const SERVER_CONFIG_FILE: &str = "conf/server_config.toml";

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub ip: String,
    pub port: Option<u16>,
    // keys: Keys,
}

#[derive(Deserialize)]
struct Keys {}

impl PartialEq for ServerConfig {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip && self.port == other.port
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::get_config;

    const TEST_SERVER_CONFIG_FILE_PATH: &str = "test_files/server_config.toml";

    impl Default for ServerConfig {
        fn default() -> Self {
            ServerConfig {
                ip: "127.0.0.1".to_string(),
                port: Some(8080),
            }
        }
    }

    #[test]
    fn parse_config_file() {
        let server_config: ServerConfig = get_config(TEST_SERVER_CONFIG_FILE_PATH).unwrap();

        assert_eq!(server_config, ServerConfig::default());
    }
}
