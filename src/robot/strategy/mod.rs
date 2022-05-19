pub mod simple_increase_decrease;

mod strategy;

pub use simple_increase_decrease::SimpleIncreaseDecreaseStrategy;
pub use strategy::{Action, OrderType, Strategy, StrategyParams};
