use crate::context_manager::{ActiveOrder, FilledOrder};
use crate::robot::strategy::StrategyParams;
use serde;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum OrderManagerState {
    Started,
    Stopped,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderParams {
    LimitOrder(LimitOrder),
    MarketOrder(MarketOrder),
    CancelOrder(CancelOrder),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderRequestType {
    Limit,
    Market,
    Cancel,
}

#[derive(Debug, PartialEq)]
pub enum OrderMsg {
    OrderContainers(Vec<OrderContainer>),
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderContainer {
    // Robot's ID which sent an order
    pub robot_id: String,

    // Order body
    pub order: Order,

    // Metainfo about strategy params
    pub metainfo: StrategyParams,

    // Estimate time
    #[serde(skip)]
    #[serde(default = "Instant::now")]
    pub created_at: Instant,
}

// Structure for serialize and deserialize sent order containers on start/stop platform
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct SentOrderContainer {
    pub robot_id: String,

    pub order: Order,

    pub metainfo: StrategyParams,
}

impl Default for OrderContainer {
    fn default() -> Self {
        OrderContainer {
            robot_id: "StubRobot".to_string(),
            order: Order::LimitOrder(LimitOrder::default()),
            metainfo: StrategyParams::Stub,
            created_at: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderMetaInfo {
    ArbitrationStrategy(ArbitrationStrategy),
    Stub,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArbitrationStrategy {
    pub axis_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Order {
    LimitOrder(LimitOrder),
    MarketOrder(MarketOrder),
    CancelOrder(CancelOrder),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimitOrder {
    pub gateway: String,
    pub symbol: String,
    pub amount: f64,
    pub price: f64,
    pub order_side: OrderSide,
    pub custom_order_id: String,
}

impl Default for LimitOrder {
    fn default() -> Self {
        LimitOrder {
            gateway: "Binance".to_string(),
            symbol: "BTCUSDT".to_string(),
            amount: 1.,
            price: 10.,
            order_side: OrderSide::Buy,
            custom_order_id: "Custom_Order_ID".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketOrder {
    pub gateway: String,
    pub symbol: String,
    pub amount: f64,
    pub order_side: OrderSide,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelOrder {
    // Primary field
    pub order_id: u64,

    // Secondary fields,
    pub gateway: String,
    pub symbol: String,
    pub price: f64,
    pub amount: f64,
    pub order_side: OrderSide,
    pub custom_order_id: String,
}

impl Default for CancelOrder {
    fn default() -> Self {
        CancelOrder {
            // robot_id: "Robot1".to_string(),
            gateway: "Binance".to_string(),
            symbol: "BTCUSDT".to_string(),
            order_id: 1213,
            amount: 1.,
            price: 1.,
            order_side: OrderSide::Buy,
            custom_order_id: "Custom Order ID".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenOrder {
    pub robot_id: String,
    pub gateway: String,
    pub symbol: String,
    pub amount: f64,
    pub price: f64,
    pub order_side: OrderSide,
    pub custom_order_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl Default for Order {
    fn default() -> Self {
        Order::LimitOrder(LimitOrder {
            // robot_id: "Robot_Huobi_1_BTC".to_string(),
            gateway: "Binance".to_string(),
            symbol: "BTCUSDT".to_string(),
            amount: 1.,
            price: 10.,
            order_side: OrderSide::Buy,
            custom_order_id: "Custom Order ID".to_string(),
        })
    }
}

pub enum ActiveOrderMsg {
    ActiveStateOrder(ActiveOrder),
    FilledOrder(FilledOrder),
}

enum Process {
    Stop,
    Continue,
}
