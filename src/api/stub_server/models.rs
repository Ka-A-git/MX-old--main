use serde::de::{self, Unexpected, Visitor};
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use std::fmt;

pub type ExchangeApiResult<T> = Result<T, &'static str>;

#[derive(Debug, Clone)]
pub struct Transaction {
    pub symbol: String,
    pub order_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct APIErrorResponse<R> {
    pub status: Option<String>,

    pub err_code: Option<u32>,

    pub err_msg: Option<String>,

    pub ts: Option<u64>,

    pub data: Option<R>,

    pub tick: Option<R>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResultSymbol {
    pub status: String,
    pub data: Vec<Symbol>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountsResult {
    data: Vec<Account>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    id: u64,
    state: String,
    #[serde(rename = "type")]
    type_: String,
    subtype: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenOrdersResult {
    pub data: Vec<OpenOrder>,
}
impl Default for OpenOrdersResult {
    fn default() -> Self {
        OpenOrdersResult { data: vec![] }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenOrder {
    pub id: u64,
    #[serde(rename = "client-order-id")]
    pub client_order_id: String,
    pub symbol: String,
    pub price: String,
    pub amount: String,
    #[serde(rename = "created-at")]
    pub created_at: u64,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "filled-amount")]
    pub filled_amount: String,
    #[serde(rename = "filled-cash-amount")]
    pub filled_cash_amount: String,
    #[serde(rename = "filled-fees")]
    pub filled_fees: String,
    pub source: String,
    pub state: String,
    // #[serde(rename = "stop-price")]
    // stop_price: String,
    // operator: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaceOrderResult {
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CancelOrderResult {
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CancelOrderWithIdResult {
    pub data: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BalanceResult {
    pub data: BalanceData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BalanceData {
    pub id: u32,
    #[serde(rename = "type")]
    pub account_type: String,
    pub state: String,
    pub list: Vec<Asset>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Asset {
    pub currency: String,

    #[serde(rename = "type")]
    pub trade_type: String,

    #[serde(deserialize_with = "string_as_f64")]
    pub balance: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TradeHistoryResult {
    pub data: Vec<TradeHistory>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TradeHistory {
    pub id: u64,
    pub symbol: String,
    #[serde(rename = "account-id")]
    pub account_id: u32,
    pub amount: String,
    pub price: String,
    #[serde(rename = "created-at")]
    pub created_at: u64,
    #[serde(rename = "type")]
    pub type_order: String,
    #[serde(rename = "field-amount")]
    pub field_amount: String,
    #[serde(rename = "field-cash-amount")]
    pub field_cash_amount: String,
    #[serde(rename = "field-fees")]
    pub field_fees: String,
    #[serde(rename = "finished-at")]
    pub finished_at: u64,
    pub source: String,
    pub state: String,
    #[serde(rename = "canceled-at")]
    pub canceled_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Symbol {
    #[serde(rename = "base-currency")]
    pub base: String,
    #[serde(rename = "quote-currency")]
    pub quote: String,
    #[serde(rename = "price-precision")]
    pub price_precision: u8,
    #[serde(rename = "amount-precision")]
    pub amount_precision: u8,
    #[serde(rename = "symbol-partition")]
    pub partition: String,
    pub symbol: String,
    pub state: String,
    #[serde(rename = "value-precision")]
    pub value_precision: u8,
    #[serde(rename = "min-order-amt")]
    pub min_amount: f64,
    #[serde(rename = "max-order-amt")]
    pub max_amount: f64,
    #[serde(rename = "min-order-value")]
    pub min_value: f64,
    #[serde(default, rename = "leverage-ratio")]
    pub max_leverage: f32,
}

fn string_as_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(F64Visitor)
}

struct F64Visitor;
impl<'de> Visitor<'de> for F64Visitor {
    type Value = f64;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string representation of a f64")
    }
    fn visit_str<E>(self, value: &str) -> Result<f64, E>
    where
        E: de::Error,
    {
        if let Ok(integer) = value.parse::<i32>() {
            Ok(integer as f64)
        } else {
            value.parse::<f64>().map_err(|_err| {
                E::invalid_value(Unexpected::Str(value), &"a string representation of a f64")
            })
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderSubs {
    pub action: String,
    pub ch: String,
    pub data: EventType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum EventType {
    Creation(Creation),
    Cancellation(Cancellation),
    Trade(Trade),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Creation {
    pub order_size: String,
    pub order_create_time: u64,
    pub account_id: u32,
    pub order_price: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub order_id: u64,
    pub client_order_id: String,
    pub order_source: String,
    pub order_status: String,
    pub symbol: String,
    pub event_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Cancellation {
    pub last_act_time: u64,
    pub remain_amt: String,
    pub exec_amt: String,
    pub order_id: u64,
    #[serde(rename = "type")]
    pub type_: String,
    pub client_order_id: String,
    pub order_source: String,
    pub order_price: String,
    pub order_size: String,
    pub order_status: String,
    pub symbol: String,
    pub event_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub trade_price: String,
    pub trade_volume: String,
    pub trade_id: u64,
    pub trade_time: u64,
    pub aggressor: bool,
    pub remain_amt: String,
    pub exec_amt: String,
    pub order_id: u64,
    #[serde(rename = "type")]
    pub type_: String,
    pub client_order_id: String,
    pub order_source: String,
    pub order_price: String,
    pub order_size: String,
    pub order_status: String,
    pub symbol: String,
    pub event_type: String,
}
