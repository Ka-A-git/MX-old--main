#![feature(drain_filter)]
#![allow(dead_code)]

mod api;
mod cli;
mod config;
mod context_manager;
mod demo;
mod gateway;
mod logger;
mod math;
mod order_manager;
mod platform;
mod robot;
mod server;
mod storage;

pub use cli::CLI;
pub use platform::Platform;
pub use server::paths;
pub use server::Server;

pub use order_manager::{Order, OrderManager};

pub use server::PLATFORM;

pub use robot::config::RobotConfig;
pub use robot::strategy::arbitration::config::ArbitrationStrategyConfig;
pub use robot::strategy::simple_increase_decrease::config::SimpleIncreaseDecreaseStrategyConfig;
pub use robot::{RobotParams, RobotParamsActions};

pub use gateway::{Gateway, GatewayParams, GatewayParamsActions};

pub use logger::Logger;
