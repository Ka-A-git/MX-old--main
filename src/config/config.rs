use serde::de::DeserializeOwned;
use std::fs;
use tracing::{error, info};

pub fn get_config<T: DeserializeOwned>(config_file_path: &str) -> Result<T, String> {
    match fs::read_to_string(config_file_path) {
        Ok(config_string) => {
            info!("Config found at {} path", config_file_path);
            match toml::from_str::<T>(&config_string) {
                Ok(config) => {
                    info!("Config {} was read successfully", config_file_path);
                    Ok(config)
                }
                Err(e) => {
                    error!("Config {} file has an error", config_file_path);
                    Err(format!("Config file error: {}", e))
                }
            }
        }
        Err(_) => Err(format!("Config file {} not found", config_file_path)),
    }
}

pub trait ParseConfig {
    fn from_file<T: DeserializeOwned>(file_path: &str) -> Result<T, String> {
        get_config(file_path)
    }
}
