use serde::{Deserialize, Serialize}; 
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher}; 

use crate::storage::{self, sensors::InfluxPoint};
use crate::{context_manager::ContextInfo, order_manager::OrderSide};

#[derive(Debug, Clone, PartialEq)]
pub enum OrderType {
    Market(Market), 
    Limit(Limit),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Market {}

#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub price: f64,
}

#[derive(Debug)]
pub struct Action {
    pub amount: f64,
    pub symbol: String,
    pub exchange: String,
    pub order_type: OrderType,
    pub order_side: OrderSide,
    pub extended_strategy_params: StrategyParams,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyParams {
    ArbitrationParams(ArbitrationParams),
    SimpleStrategyParams(SimpleStrategyParams),
    Stub,
}

#[derive(Debug, Clone, Hash, PartialEq, Serialize, Deserialize)]
pub struct ArbitrationParams {
    pub axes_id: String,
    pub level: String,
}

impl StrategyHash for ArbitrationParams {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimpleStrategyParams {
    pub name: String,
}

impl Default for Action {
    fn default() -> Self {
        Action {
            amount: 1.1,
            symbol: "BTCUSDT".to_string(),
            exchange: "Binance".to_string(),
            order_type: OrderType::Limit(Limit { price: 1.1 }),
            order_side: OrderSide::Buy,
            extended_strategy_params: StrategyParams::ArbitrationParams(ArbitrationParams {
                axes_id: "Binance".to_string(),
                level: "1".to_string(),
            }),
        }
    }
}

impl PartialEq for Action {
    fn eq(&self, other: &Self) -> bool {
        self.order_side == other.order_side
            && self.amount == other.amount
            && self.symbol == other.symbol
            && self.exchange == other.exchange
            && self.order_type == other.order_type
    }
}

pub trait StrategyHash: Hash {
    fn get_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub trait Strategy: Sync + Send + std::fmt::Debug {
    fn start(&self) -> Result<(), &'static str>;

    fn load_data(&self, context_info: ContextInfo) -> Result<(), &'static str>;

    fn calc(&self) -> Result<(Vec<Action>, Vec<InfluxPoint>), &'static str>;

    fn get_data(&self) -> Result<ContextInfo, &'static str>;

    fn buy(&self) -> bool;

    fn sell(&self) -> bool;

    fn clear_data(&self) -> Result<(), &'static str>;

    fn finish(&self) -> Result<(), &'static str>;

    fn sense(&mut self) -> Vec<storage::SensorMsg> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_hash() {
        let strategy_params = ArbitrationParams {
            axes_id: "Binance".to_string(),
            level: "0".to_string(),
        };

        let hash = strategy_params.get_hash();

        println!("Hash: {}", hash);
    }
}
