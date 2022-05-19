use crate::config::config::ParseConfig;
use crate::gateway::GatewayController;
use crate::platform::config::PlatformConfig;
use crate::platform::PlatforomController;
use crate::robot::RobotController;
use actix_web::web;
use actix_web::{HttpRequest, Responder};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigRequestParams {
    name: String,
    config_file_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlatformConfigRequestParams {
    config_file_path: String,
}

fn check_config_file_path(file_path: &str) -> Result<(), &str> {
    let path = Path::new(file_path);
    if path.exists() {
        if let Some(ext) = path.extension() {
            if ext == "toml" {
                Ok(())
            } else {
                Err("File has wrong extention")
            }
        } else {
            Err("File doen't have extention")
        }
    } else {
        Err("File config not found")
    }
}

pub async fn home(_req: HttpRequest) -> impl Responder {
    format!("See README for commands")
}

pub async fn platform_start(_req: HttpRequest) -> impl Responder {
    PlatforomController::start()
}

pub async fn platform_stop(_req: HttpRequest) -> impl Responder {
    PlatforomController::stop()
}

pub async fn platform_status(_req: HttpRequest) -> impl Responder {
    PlatforomController::status()
}

pub async fn platform_info(_req: HttpRequest) -> impl Responder {
    PlatforomController::info()
}

pub async fn platform_set_config(
    _req: HttpRequest,
    params: web::Form<PlatformConfigRequestParams>,
) -> impl Responder {
    let platform_config_file_path = &params.config_file_path;
    match check_config_file_path(platform_config_file_path) {
        Ok(_) => {
            let platform_config = PlatformConfig::from_file(platform_config_file_path).unwrap();

            PlatforomController::set_config(platform_config)
        }
        Err(error) => format!("Config error: {}", error),
    }
}

pub async fn robot_start(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap();

    RobotController::start(name)
}

pub async fn robot_stop(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap();

    RobotController::stop(name)
}

pub async fn robot_status(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap();
    RobotController::status(name)
}

pub async fn robot_info(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap();
    RobotController::info(name)
}

pub async fn robot_set_config(
    _req: HttpRequest,
    params: web::Form<ConfigRequestParams>,
) -> impl Responder {
    let robot_config_file_path = &params.config_file_path;
    match check_config_file_path(robot_config_file_path) {
        Ok(_) => RobotController::set_config(&params.name, robot_config_file_path),
        Err(error) => format!("Config error: {}", error),
    }
}

pub async fn robot_up(_req: HttpRequest) -> impl Responder {
    RobotController::up()
}

pub async fn robot_list(_req: HttpRequest) -> impl Responder {
    RobotController::list()
}

pub async fn gateway_start(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap();
    GatewayController::start(name)
}

pub async fn gateway_stop(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap();

    GatewayController::stop(name)
}

pub async fn gateway_status(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap();
    GatewayController::status(name)
}

pub async fn gateway_info(req: HttpRequest) -> impl Responder {
    // req.app_data();
    let name = req.match_info().get("name").unwrap();

    GatewayController::info(name)
}

pub async fn gateway_set_config(
    _req: HttpRequest,
    params: web::Form<ConfigRequestParams>,
) -> impl Responder {
    let gateway_config_file_path = &params.config_file_path;

    match check_config_file_path(gateway_config_file_path) {
        Ok(_) => GatewayController::set_config(&params.name, gateway_config_file_path),
        Err(error) => format!("Config error: {}", error),
    }
}

pub async fn gateway_up(_req: HttpRequest) -> impl Responder {
    GatewayController::up()
}

pub async fn gateway_list(_req: HttpRequest) -> impl Responder {
    GatewayController::list()
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_robot_start() {}

    #[test]
    fn test_robot_staus() {}
}
