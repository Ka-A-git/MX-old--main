use crate::server::PLATFORM;
use tracing::{error, info};

pub struct GatewayController;

impl GatewayController {
    pub fn start(gateway_name: &str) -> String {
        info!("Gateway {} is starting...", gateway_name);

        match PLATFORM
            .environment
            .gateway_environment
            .start_gateway(gateway_name)
        {
            Ok(_) => format!("Gateway {} has been started", gateway_name),
            Err(error) => format!("{}", error),
        }
    }

    pub fn stop(gateway_name: &str) -> String {
        info!("Gateway {} is stopping...", gateway_name);

        let dependent_robots = PLATFORM
            .environment
            .gateway_environment
            .get_dependent_robots(gateway_name);

        // Stop all dependent robots on that gateway
        for robot_name in dependent_robots {
            match PLATFORM
                .environment
                .robot_environment
                .stop_robot(&robot_name)
            {
                Ok(_) => info!("Robot {} was stopped", robot_name),
                Err(_) => error!("Robot {} wasn't stopped", robot_name),
            }
        }

        match PLATFORM
            .environment
            .gateway_environment
            .stop_gateway(gateway_name)
        {
            Ok(_) => format!("Gateway {} has been stopped", gateway_name),
            Err(error) => format!("{}", error),
        }
    }

    pub fn status(gateway_name: &str) -> String {
        info!("Gateway {} status is {}", gateway_name, "[get status]");

        match PLATFORM
            .environment
            .gateway_environment
            .status_gateway(gateway_name)
        {
            Ok(status) => format!("The gateway {} is {}", gateway_name, status),
            Err(error) => format!("{}", error),
        }
    }

    pub fn info(gateway_name: &str) -> String {
        info!("Getting info of the {} gateway", gateway_name);

        match PLATFORM
            .environment
            .gateway_environment
            .info_gateway(gateway_name)
        {
            Ok(info) => format!("Gateway {} info:\n {}", gateway_name, info),
            Err(error) => format!("{}", error),
        }
    }

    pub fn set_config(gateway_name: &str, config_file_path: &str) -> String {
        info!("Setting config for the {} gateway", gateway_name);

        match PLATFORM
            .environment
            .gateway_environment
            .set_config_gateway(gateway_name, config_file_path)
        {
            Ok(_) => format!(
                "The config of the Gateway {} has been changed\n Config: {}",
                gateway_name, config_file_path
            ),
            Err(error) => format!("{}", error),
        }
    }

    pub fn up() -> String {
        info!("All gateways are starting...");

        match PLATFORM.environment.gateway_environment.up_gateways() {
            Ok(_) => format!("All gateways have been started"),
            Err(error) => format!("{}", error),
        }
    }

    pub fn list() -> String {
        info!("Gateway list");

        match PLATFORM.environment.gateway_environment.list_gateways() {
            Ok(gateways) => format!("Gateway list:\n {}", gateways.join("\n")),
            Err(error) => format!("{}", error),
        }
    }
}
