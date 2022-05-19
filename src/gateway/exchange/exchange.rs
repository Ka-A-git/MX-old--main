use crate::gateway::{gateway::ExchangeInstrumentInfo, Depth, Instrument};
use std::collections::HashMap;

pub type ExchangeApiResult<T> = Result<T, &'static str>;
pub trait ExchangeAction: Sync + Send {
    fn inti(&self);

    fn fetch_metadata(&self) -> Vec<ExchangeInstrumentInfo>;

    fn fetch_depth(&self, symbol: &str) -> Result<Depth, &'static str>;

    fn fetch_balances(
        &self,
        instruments: Vec<Instrument>,
    ) -> Result<HashMap<String, f64>, &'static str>;

    fn limit_buy(
        &self,
        symbol: &str,
        amount: f64,
        price: f64,
        custom_order_id: Option<String>,
    ) -> ExchangeApiResult<PlatformTransaction>;

    fn limit_sell(
        &self,
        symbol: &str,
        amount: f64,
        price: f64,
        custom_order_id: Option<String>,
    ) -> ExchangeApiResult<PlatformTransaction>;

    fn market_buy(&self, symbol: &str, amount: f64) -> ExchangeApiResult<PlatformTransaction>;

    fn market_sell(&self, symbol: &str, amount: f64) -> ExchangeApiResult<PlatformTransaction>;

    fn cancel_order(
        &self,
        symbol: &str,
        custom_order_id: &str,
    ) -> ExchangeApiResult<PlatformTransaction>;
}

#[derive(Debug, Clone)]
pub struct PlatformTransaction {
    pub symbol: String,
    pub order_id: u64,
}

impl Default for PlatformTransaction {
    fn default() -> Self {
        PlatformTransaction {
            symbol: "BTCUSDT".to_string(),
            order_id: 0,
        }
    }
}

#[cfg(test)]
mod tests {}
