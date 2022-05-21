use crate::server::PLATFORM; 
use tracing::info;  
  
pub struct RobotController; 

impl RobotController {
    pub fn start(robot_name: &str) -> String {
        info!("Robot {} is starting...", robot_name);

        match PLATFORM
            .environment
            .robot_environment
            .start_robot(robot_name)
        {
            Ok(_) => format!("Robot {} has been started", robot_name),
            Err(error) => format!("{}", error),
        }
    }

    pub fn stop(robot_name: &str) -> String {
        info!("Robot {} is stopping...", robot_name);

        match PLATFORM
            .environment
            .robot_environment
            .stop_robot(robot_name)
        {
            Ok(_) => format!("Robot {} has been stopped", robot_name),
            Err(error) => format!("{}", error),
        }
    } 

    pub fn status(robot_name: &str) -> String {
        info!("Get status of the {} robot", robot_name);

        match PLATFORM
            .environment
            .robot_environment
            .status_robot(robot_name)
        {
            Ok(status) => format!("The status of the robot {} is {}", robot_name, status),
            Err(error) => format!("{}", error),
        }
    }

    pub fn info(robot_name: &str) -> String {
        info!("Getting info of the {} robot", robot_name);

        match PLATFORM
            .environment
            .robot_environment
            .info_robot(robot_name)
        {
            Ok(info) => format!("Robot {} info:\n {}", robot_name, info),
            Err(error) => format!("{}", error),
        }
    }

    pub fn set_config(robot_name: &str, config_file_path: &str) -> String {
        info!("Setting config for the {} robot", robot_name);

        match PLATFORM
            .environment
            .robot_environment
            .set_config_robot(robot_name, config_file_path)
        {
            Ok(_) => format!(
                "The config of the Robot {} has been changed\n Config: {}",
                robot_name, config_file_path
            ),
            Err(error) => format!("{}", error),
        }
    }

    pub fn up() -> String {
        info!("All robots are starting...");

        match PLATFORM.environment.robot_environment.start_all_robots() {
            Ok(_) => format!("All robots have been started"),
            Err(error) => format!("{}", error),
        }
    }

    pub fn list() -> String {
        info!("Robot list");

        match PLATFORM.environment.robot_environment.list_robots() {
            Ok(robots) => format!("Robot list:\n {:?}", robots.join("\n")),
            Err(error) => format!("{}", error),
        }  
    }
} 
