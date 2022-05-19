use crate::config::ParseConfig;
use serde_derive::Deserialize;
use std::{cmp::PartialEq, net::SocketAddr};

pub const PLATFORM_CONFIG_FILE_PATH: &str = "conf/platform_config.toml";

#[derive(Deserialize, Debug, Clone)]
pub struct Robot {
    pub name: String,
    pub config_file_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Gateway {
    pub name: String,
    pub config_file_path: String,
}

#[derive(Deserialize, Debug)]
pub struct InfluxDb {
    pub host_address: SocketAddr,
}

#[derive(Deserialize, Debug)]
pub struct PlatformConfig {
    pub robots: Vec<Robot>,
    pub gateways: Vec<Gateway>,
    pub influxdb: InfluxDb,
}

impl PlatformConfig {
    pub fn get_config(config_file_path: &str) -> Self {
        PlatformConfig::from_file(config_file_path).unwrap()
    }
}

impl ParseConfig for PlatformConfig {}

impl PartialEq for PlatformConfig {
    fn eq(&self, other: &Self) -> bool {
        self.robots == other.robots && self.gateways == other.gateways
    }
}

impl PartialEq for Robot {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.config_file_path == other.config_file_path
    }
}

impl PartialEq for Gateway {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.config_file_path == other.config_file_path
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::get_config;

    const TEST_DIR: &str = "test_files";

    const TEST_PLATFORM_CONFIG_FILE_PATH: &str = "test_files/platform_config.toml";

    impl Default for PlatformConfig {
        fn default() -> Self {
            let host_address: SocketAddr = "18.132.99.157:8094".parse().unwrap();

            Self {
                robots: vec![
                    Robot {
                        name: "Robot_Huobi_1".to_string(),
                        config_file_path: format!("{}/robot_huobi_1_config.toml", TEST_DIR),
                    },
                    Robot {
                        name: "Robot_Huobi_2".to_string(),
                        config_file_path: format!("{}/robot_huobi_2_config.toml", TEST_DIR),
                    },
                    Robot {
                        name: "Robot_Binance".to_string(),
                        config_file_path: format!("{}/robot_binance_config.toml", TEST_DIR),
                    },
                ],
                gateways: vec![
                    Gateway {
                        name: "Huobi".to_string(),
                        config_file_path: format!("{}/gateway_huobi_config.toml", TEST_DIR),
                    },
                    Gateway {
                        name: "Binance".to_string(),
                        config_file_path: format!("{}/gateway_binance_config.toml", TEST_DIR),
                    },
                ],

                influxdb: InfluxDb { host_address },
            }
        }
    }

    #[test]
    fn parse_config_file() {
        let platform_config: PlatformConfig = get_config(TEST_PLATFORM_CONFIG_FILE_PATH).unwrap();

        assert_eq!(platform_config, PlatformConfig::default());
    }
}
