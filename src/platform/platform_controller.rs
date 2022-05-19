use super::config::PlatformConfig;
use crate::server::PLATFORM;
use tracing::info;

pub struct PlatforomController;

impl PlatforomController {
    pub fn start() -> String {
        info!("Starting the Platform");

        match PLATFORM.start() {
            Ok(_) => "Platform has been started".to_string(),
            Err(error) => format!("{}", error),
        }
    }

    pub fn stop() -> String {
        info!("Stopipng the Platform");

        match PLATFORM.stop() {
            Ok(_) => "Platform has been stopped".to_string(),
            Err(error) => format!("{}", error),
        }
    }

    pub fn status() -> String {
        info!("Getting status of the Platform");

        match PLATFORM.status() {
            Ok(status) => format!("Platform status: {}", status),
            Err(error) => format!("{}", error),
        }
    }

    pub fn info() -> String {
        info!("Getting info of the Platform");

        match PLATFORM.info() {
            Ok(info) => format!("Platform info:\n\n{}", info.join("\n")),
            Err(error) => format!("{}", error),
        }
    }

    pub fn set_config(_config: PlatformConfig) -> String {
        info!("Setting config of the Platform");

        // TODO

        "The config of the Platform has been changed".to_string()
    }
}
