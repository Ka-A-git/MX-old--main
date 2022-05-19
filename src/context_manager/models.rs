use crate::gateway::{Depth, OrderBook};
use crate::order_manager::OrderSide;
use crate::robot::strategy::{ArbitrationParams, StrategyParams};
use serde::{Deserialize, Serialize};
use std::time::Instant;

// Gateway can send this kind of messages
#[derive(Clone, Debug)]
pub enum GatewayMsg {
    DepthMsg(DepthMsg),
    ActiveOrder(ActiveOrder),
    FilledOrder(FilledOrder),
}

#[derive(Clone, Debug)]
pub struct DepthMsg {
    pub depth_info: DepthInfo,
    pub created_at: Instant,
}

#[derive(Clone, Debug)]
pub struct DepthInfo {
    pub gateway_name: String,
    pub exchange_name: String,
    pub symbol: String,
    pub depth: Depth,
}

// Order that was successfully sent to the exchange and its response returned to gateway
#[derive(Clone, Debug)]
pub struct ActiveOrder {
    pub custom_order_id: String,
    pub robot_id: String,
    pub gateway: String,
    pub symbol: String,
    pub amount: f64,
    pub price: f64,
    pub order_side: OrderSide,
    pub strategy_params: StrategyParams,
}

impl Default for ActiveOrder {
    fn default() -> Self {
        ActiveOrder {
            custom_order_id: "Custom123".to_string(),
            robot_id: "RobotStub".to_string(),
            gateway: "Binance".to_string(),
            symbol: "BTCUSDT".to_string(),
            amount: 0.001,
            price: 30000.,
            order_side: OrderSide::Buy,
            strategy_params: StrategyParams::ArbitrationParams(ArbitrationParams {
                axes_id: "Binance".to_string(),
                level: "Overlap".to_string(),
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FilledOrder {
    pub custom_order_id: String,
    pub order_id: u64,
    pub symbol: String,
    pub amount: String,
}

impl Default for FilledOrder {
    fn default() -> Self {
        FilledOrder {
            custom_order_id: "Custom123".to_string(),
            order_id: 123,
            symbol: "BTCUSDT".to_string(),
            amount: "0.001".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilledInfo {
    pub order_id: u64,
    pub custom_order_id: String,
    pub gateway: String,
    pub robot_id: String,
    pub symbol: String,
    pub amount: String,
    pub price: f64,
    pub order_side: OrderSide,
    pub strategy_params: StrategyParams,
}

// Message that is sent to robots
#[derive(Clone, Debug)]
pub enum ContextMsg {
    ContextInfo(ContextInfo),
}

#[derive(Clone, Debug)]
pub struct ContextInfo {
    pub orderbooks_info: Vec<OrderBookInfo>,

    pub positions: Vec<Position>,

    pub created_at: Instant,
}

impl ContextInfo {
    pub fn new() -> Self {
        Self {
            orderbooks_info: Vec::new(),
            positions: Vec::new(),
            created_at: Instant::now(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Position {
    pub gateway: String,
    pub symbol: String,
    pub amount: f64,
    pub price: f64,
    pub order_side: OrderSide,
    pub strategy_params: StrategyParams,
}

#[derive(Clone, Debug)]
pub struct OrderBookInfo {
    pub gateway_name: String,
    pub exchange_name: String,
    pub symbol: String,
    pub order_book: OrderBook,
}

impl Default for GatewayMsg {
    fn default() -> Self {
        GatewayMsg::DepthMsg(DepthMsg {
            depth_info: DepthInfo {
                gateway_name: "GatewayStub".to_string(),
                exchange_name: "ExchangeStub".to_string(),
                symbol: "BTCUSDT".to_string(),
                depth: Depth::default(),
            },
            created_at: Instant::now(),
        })
    }
}

impl Default for ContextMsg {
    fn default() -> Self {
        ContextMsg::ContextInfo(ContextInfo::default())
    }
}

impl Default for OrderBookInfo {
    fn default() -> Self {
        OrderBookInfo {
            gateway_name: "GatewayStub".to_string(),
            exchange_name: "ExchangeStub".to_string(),
            symbol: "BTCUSDT".to_string(),
            order_book: OrderBook::default(),
        }
    }
}

impl Default for ContextInfo {
    fn default() -> Self {
        ContextInfo {
            orderbooks_info: vec![OrderBookInfo::default()],
            positions: Vec::new(),
            created_at: Instant::now(),
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Position {
            gateway: "Binance".to_string(),
            symbol: "BTCUSDT".to_string(),
            amount: 1.,
            price: 1.,
            order_side: OrderSide::Buy,
            strategy_params: StrategyParams::Stub,
        }
    }
}

impl Position {
    pub fn init_bid_stub_position(price: f64) -> Self {
        Self {
            price,
            order_side: OrderSide::Sell,
            ..Default::default()
        }
    }

    pub fn init_ask_stub_position(price: f64) -> Self {
        Self {
            price,
            order_side: OrderSide::Buy,
            ..Default::default()
        }
    }
}
