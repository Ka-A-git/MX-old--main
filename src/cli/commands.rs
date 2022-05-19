use super::config::{CLIConfig, CLI_CONFIG_FILE};
use super::requests::request::Request;
use crate::config::get_config;
use crate::paths::*;
use async_trait::async_trait;
use std::marker::Send;

pub trait Command: Send {
    fn get_connection() -> Request {
        let config: CLIConfig = get_config(CLI_CONFIG_FILE).unwrap();
        let base_url = format!("http://{}:{}", config.ip, config.port.unwrap());
        let request = Request::new(&base_url);
        request
    }
}

#[async_trait]
pub trait GetReq: Command {
    async fn get_request(endpoint: &str) {
        match Self::get_connection().get_request(endpoint).await {
            Ok(responce) => {
                println!("{}", responce);
            }
            Err(error) => {
                eprintln!("Request error {:?}", error);
            }
        }
    }
}

#[async_trait]
pub trait PostReq: Command {
    async fn post_request(endpoint: &str, params: &[(&str, &str)]) {
        match Self::get_connection().post_request(endpoint, params).await {
            Ok(responce) => {
                println!("{}", responce);
            }
            Err(error) => {
                eprintln!("Request error {:?}", error);
            }
        }
    }
}

#[async_trait]
pub trait PlatformActionCommand: PostReq {
    async fn start();

    async fn stop();
}

#[async_trait]
pub trait ActionCommand: PostReq {
    async fn start(name: Option<&str>);

    async fn stop(name: Option<&str>);

    async fn action_command(endpoint: &str, name: Option<&str>) {
        Self::post_request(&endpoint.replace("{name}", name.unwrap()), &[]).await;
    }
}

#[async_trait]
pub trait InfoCommand: GetReq {
    async fn info_command(endpoint: &str, name: Option<&str>) {
        Self::get_request(&endpoint.replace("{name}", name.unwrap())).await;
    }

    async fn info(name: Option<&str>);
}

#[async_trait]
pub trait UpCommand: PostReq {
    async fn up();

    async fn up_command(endpoint: &str) {
        Self::post_request(endpoint, &[]).await;
    }
}

#[async_trait]
pub trait ListCommand: GetReq {
    async fn list_command(endpoint: &str) {
        Self::get_request(endpoint).await;
    }
    async fn list();
}

#[async_trait]
pub trait PlatformStatusCommand: GetReq {
    async fn status_command(endpoint: &str) {
        Self::get_request(endpoint).await;
    }
    async fn status();
}

#[async_trait]
pub trait StatusCommand: GetReq {
    async fn status_command(endpoint: &str, name: Option<&str>) {
        Self::get_request(&endpoint.replace("{name}", name.unwrap())).await;
    }
    async fn status(name: Option<&str>);
}

#[async_trait]
pub trait SetConfig: PostReq {
    async fn set_config_command(endpoint: &str, name: Option<&str>, config_file_path: &str) {
        let params = [
            ("name", name.unwrap()),
            ("config_file_path", config_file_path),
        ];
        Self::post_request(&endpoint.replace("{name}", name.unwrap()), &params).await;
    }

    async fn set_config(name: Option<&str>, file_path_config: &str);
}

pub struct PlatformCommand;

impl Command for PlatformCommand {}
impl GetReq for PlatformCommand {}
impl PostReq for PlatformCommand {}

#[async_trait]
impl PlatformActionCommand for PlatformCommand {
    async fn start() {
        println!("Starting platform...");
        Self::post_request(PLATFORM_START, &[]).await;
    }

    async fn stop() {
        println!("Stoping platform...");
        Self::post_request(PLATFORM_STOP, &[]).await;
    }
}

#[async_trait]
impl PlatformStatusCommand for PlatformCommand {
    async fn status() {
        println!("Getting platform status ...");
        Self::status_command(PLATFORM_STATUS).await;
    }
}

impl PlatformCommand {
    pub async fn set_config(config_file_path: &str) {
        println!("Setting platform configuration...");
        let params = [("config_file_path", config_file_path)];
        Self::post_request(PLATFORM_SET_CONFIG, &params).await;
    }
}

pub struct RobotCommand;

impl Command for RobotCommand {}
impl GetReq for RobotCommand {}
impl PostReq for RobotCommand {}

#[async_trait]
impl ActionCommand for RobotCommand {
    async fn start(name: Option<&str>) {
        println!("Starting robot {}...", name.unwrap());
        Self::action_command(ROBOT_START, name).await;
    }

    async fn stop(name: Option<&str>) {
        println!("Stoping robot {}...", name.unwrap());
        Self::action_command(ROBOT_STOP, name).await;
    }
}

#[async_trait]
impl StatusCommand for RobotCommand {
    async fn status(name: Option<&str>) {
        println!("Getting status of the robot {}...", name.unwrap());
        Self::status_command(ROBOT_STATUS, name).await;
    }
}

#[async_trait]
impl InfoCommand for RobotCommand {
    async fn info(name: Option<&str>) {
        println!("Getting info of the {} robot...", name.unwrap());
        Self::info_command(ROBOT_INFO, name).await;
    }
}

#[async_trait]
impl SetConfig for RobotCommand {
    async fn set_config(name: Option<&str>, robot_config_file_path: &str) {
        println!("Setting config to {} robot...", name.unwrap());
        Self::set_config_command(ROBOT_SET_CONFIG, name, robot_config_file_path).await;
    }
}

#[async_trait]
impl UpCommand for RobotCommand {
    async fn up() {
        println!("Starting all robots");
        Self::up_command(ROBOT_UP).await;
    }
}

#[async_trait]
impl ListCommand for RobotCommand {
    async fn list() {
        println!("Getting list of robots...");
        Self::list_command(ROBOT_LIST).await;
    }
}

pub struct GatewayCommand;

impl Command for GatewayCommand {}
impl GetReq for GatewayCommand {}
impl PostReq for GatewayCommand {}

#[async_trait]
impl ActionCommand for GatewayCommand {
    async fn start(name: Option<&str>) {
        println!("Starting gateway {:?}...", name);
        Self::action_command(GATEWAY_START, name).await;
    }

    async fn stop(name: Option<&str>) {
        println!("Stoping robot {:?}...", name);
        Self::action_command(GATEWAY_STOP, name).await;
    }
}

#[async_trait]
impl StatusCommand for GatewayCommand {
    async fn status(name: Option<&str>) {
        println!("Getting status of the robot {:?}...", name);
        Self::status_command(GATEWAY_STATUS, name).await;
    }
}

#[async_trait]
impl InfoCommand for GatewayCommand {
    async fn info(name: Option<&str>) {
        println!("Getting info of the {} gateway...", name.unwrap());
        Self::info_command(GATEWAY_INFO, name).await;
    }
}

#[async_trait]
impl SetConfig for GatewayCommand {
    async fn set_config(name: Option<&str>, gateway_config_file_path: &str) {
        println!("Setting config to {:?} gateway...", name);
        Self::set_config_command(GATEWAY_SET_CONFIG, name, gateway_config_file_path).await;
    }
}

#[async_trait]
impl UpCommand for GatewayCommand {
    async fn up() {
        println!("Starting all gateways");
        Self::up_command(GATEWAY_UP).await;
    }
}

#[async_trait]
impl ListCommand for GatewayCommand {
    async fn list() {
        println!("Getting list of gateways...");
        Self::list_command(GATEWAY_LIST).await;
    }
}
