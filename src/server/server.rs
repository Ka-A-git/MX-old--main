use super::config::{ServerConfig, SERVER_CONFIG_FILE};
use super::routes::routes::*;
use crate::config::config::get_config;
use crate::logger::Logger;
use crate::platform::Platform;
use crate::storage::{self, platform::PlatformStorage, Storage, StorageConnection};
use actix_web::{web, App, HttpServer};
use ctrlc;
use lazy_static::lazy_static;
use std::sync::Mutex;
use tracing::info;

lazy_static! {
    pub static ref PLATFORM: &'static Platform = Box::leak(Box::new(Platform::init()));
}

// Routes
pub mod paths {
    pub const HOME: &str = "/";

    pub const PLATFORM_START: &str = "platform/start";
    pub const PLATFORM_STOP: &str = "platform/stop";
    pub const PLATFORM_STATUS: &str = "platform/status";
    pub const PLATFORM_INFO: &str = "platform/info";
    pub const PLATFORM_SET_CONFIG: &str = "platform/set_config";

    pub const ROBOT_START: &str = "robot/start/{name}";
    pub const ROBOT_STOP: &str = "robot/stop/{name}";
    pub const ROBOT_STATUS: &str = "robot/status/{name}";
    pub const ROBOT_INFO: &str = "robot/info/{name}";
    pub const ROBOT_SET_CONFIG: &str = "robot/set_config/{name}";
    pub const ROBOT_UP: &str = "robot/up";
    pub const ROBOT_LIST: &str = "robot/list";

    pub const GATEWAY_START: &str = "gateway/start/{name}";
    pub const GATEWAY_STOP: &str = "gateway/stop/{name}";
    pub const GATEWAY_STATUS: &str = "gateway/status/{name}";
    pub const GATEWAY_INFO: &str = "gateway/info/{name}";
    pub const GATEWAY_SET_CONFIG: &str = "gateway/set_config/{name}";
    pub const GATEWAY_UP: &str = "gateway/up";
    pub const GATEWAY_LIST: &str = "gateway/list";
}

pub struct Server {}

pub struct PlatformApp {
    pub platform: Mutex<&'static Platform>,
}

impl Server {
    async fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Init Server");
        storage::PlatformStore::new_connection().init()?;
        storage::RobotStore::new_connection().init()?;
        storage::GatewayStore::new_connection().init()?;
        Server::graceful_shutdown();
        Ok(())
    }

    pub fn graceful_shutdown() {
        ctrlc::set_handler(move || {
            // PLATFORM.read().unwrap().context_manager.graceful_shutdown();
        })
        .expect("Error setting Ctrl-C handler");
    }

    pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use paths::*;
        Logger::init();
        info!("Server is starting");
        Self::init().await?;
        let config: ServerConfig = get_config(SERVER_CONFIG_FILE).unwrap();

        // let platform_app = web::Data::new(PlatformApp {
        //     platform: Mutex::new(Box::leak(Box::new(Platform::init()))),
        // });

        HttpServer::new(move || {
            App::new()
                // .app_data(platform_app.clone())
                .route(HOME, web::get().to(home))
                .route(PLATFORM_START, web::post().to(platform_start))
                .route(PLATFORM_STOP, web::post().to(platform_stop))
                .route(PLATFORM_STATUS, web::get().to(platform_status))
                .route(PLATFORM_INFO, web::get().to(platform_info))
                .route(PLATFORM_SET_CONFIG, web::post().to(platform_set_config))
                .route(ROBOT_START, web::post().to(robot_start))
                .route(ROBOT_STOP, web::post().to(robot_stop))
                .route(ROBOT_STATUS, web::get().to(robot_status))
                .route(ROBOT_INFO, web::get().to(robot_info))
                .route(ROBOT_SET_CONFIG, web::post().to(robot_set_config))
                .route(ROBOT_UP, web::post().to(robot_up))
                .route(ROBOT_LIST, web::get().to(robot_list))
                .route(GATEWAY_START, web::post().to(gateway_start))
                .route(GATEWAY_STOP, web::post().to(gateway_stop))
                .route(GATEWAY_STATUS, web::get().to(gateway_status))
                .route(GATEWAY_INFO, web::get().to(gateway_info))
                .route(GATEWAY_SET_CONFIG, web::post().to(gateway_set_config))
                .route(GATEWAY_UP, web::post().to(gateway_up))
                .route(GATEWAY_LIST, web::get().to(gateway_list))
        })
        .bind(format!("{}:{}", config.ip, config.port.unwrap()))?
        .run()
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn init_server() {
        assert!(Server::init().await.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn run_server() {
        assert!(Server::run().await.is_ok());
    }
}
