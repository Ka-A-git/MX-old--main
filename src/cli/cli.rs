use super::commands::{self, *};
use std::io::{self, BufRead};

const HELP: &str = r#"      Available commands:
help - Print all available commands
platform
    start - Start the Platform
    stop - Stop the Platform
    status - Get status of the Platform
    config <file_path> - Set configuration for the Platform
robot
    start <robot_name> - Start the Robot by name
    stop <robot_name> - Stop the Robot by name
    status <robot_name> - Get status of the Robot by name
    info <robot_name> - Get info of the Robot by name
    config <robot_name> <file_path> - Set configuration for the Robot
    up - Start all Robots
    list - Get all available Robots on the Platform
gateway
    start <gateway_name> - Start the Gateway by name
    stop <gateway_name> - Stop the Gateway by name
    status <gateway_name> - Get status of the Gateway by name
    info <gateway_name> - Get info of the Gateway by name
    config <gateway_name> <file_path> - Set configuration for the Gateway
    up - Start all Gateways
    list - Get all available Gateways on the Platform
exit - Disconnect from Trading Platform and quit
"#;

const HELP_COMMAND: &str = "help";

pub struct CLI;

impl CLI {
    pub async fn run() {
        println!("\n\tWelcome to the CLI Trading Platform\n");

        let stdin = io::stdin();

        loop {
            let mut line = String::new();
            stdin.lock().read_line(&mut line).unwrap();
            let mut command = line.trim().split(" ");

            match command.next() {
                Some(HELP_COMMAND) => println!("{}", HELP),

                Some("exit") => {
                    println!("Exit...");
                    break;
                }

                Some("platform") => match command.next() {
                    Some("start") => {
                        commands::PlatformCommand::start().await;
                    }

                    Some("stop") => {
                        commands::PlatformCommand::stop().await;
                    }

                    Some("status") => {
                        commands::PlatformCommand::status().await;
                    }

                    Some("config") => match command.next() {
                        Some(file_path) => {
                            commands::PlatformCommand::set_config(file_path).await;
                        }
                        None => {
                            eprintln!("Command error: you should specify the file path for Platform config");
                        }
                    },
                    _ => {
                        eprintln!("Unknown command for platform");
                    }
                },

                Some("robot") => match command.next() {
                    Some("start") => {
                        let name = command.next();
                        commands::RobotCommand::start(name).await;
                    }

                    Some("stop") => {
                        let name = command.next();
                        commands::RobotCommand::stop(name).await;
                    }

                    Some("status") => {
                        let name = command.next();
                        commands::RobotCommand::status(name).await;
                    }

                    Some("info") => {
                        let name = command.next();
                        commands::RobotCommand::info(name).await;
                    }

                    Some("config") => {
                        let name = command.next();
                        match command.next() {
                            Some(file_path) => {
                                commands::RobotCommand::set_config(name, file_path).await;
                            }
                            None => {
                                eprintln!("Command error: you should specify the file path for Robot config");
                            }
                        }
                    }

                    Some("up") => {
                        commands::RobotCommand::up().await;
                    }

                    Some("list") => {
                        commands::RobotCommand::list().await;
                    }
                    _ => {
                        eprintln!("Unknown command for robot");
                    }
                },

                Some("gateway") => match command.next() {
                    Some("start") => {
                        let name = command.next();
                        commands::GatewayCommand::start(name).await;
                    }

                    Some("stop") => {
                        let name = command.next();
                        commands::GatewayCommand::stop(name).await;
                    }

                    Some("status") => {
                        let name = command.next();
                        commands::GatewayCommand::status(name).await;
                    }

                    Some("info") => {
                        let name = command.next();
                        commands::GatewayCommand::info(name).await;
                    }

                    Some("config") => {
                        let name = command.next();
                        match command.next() {
                            Some(file_path) => {
                                commands::GatewayCommand::set_config(name, file_path).await;
                            }
                            None => {
                                eprintln!("Command error: you should specify the file path for Gateway config");
                            }
                        }
                    }

                    Some("up") => {
                        commands::GatewayCommand::up().await;
                    }

                    Some("list") => {
                        commands::GatewayCommand::list().await;
                    }
                    _ => {
                        eprintln!("Unknown command for gateway");
                    }
                },

                _ => {
                    eprintln!("Unknown command for CLI");
                }
            }
        }
    }
}
