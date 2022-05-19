pub mod config;
pub mod gateway;
pub mod orderbook;
pub mod platform;
pub mod robot;
pub mod sensors;

mod storage;

pub use gateway::GatewayStore;
pub use orderbook::OrderBookStore;
pub use platform::PlatformStore;
pub use robot::RobotStore;
pub use storage::{Storage, StorageConnection};

#[derive(Debug)]
pub enum SensorMsg {
    InfluxPoint(sensors::InfluxPoint),
    Terminate,
}
