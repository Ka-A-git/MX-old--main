
use crate::api::huobi::websocket_data::HuobiWS;
use crate::gateway::{Depth, ExchangeName, GatewayParams, GatewayParamsAccount};
use binance::{self, api::Binance};
use std::collections::HashMap;
pub type ExchangeApiResult<T> = Result<T, &'static str>;

#[derive(Debug, Clone)]
pub struct Transaction {
    pub symbol: String,
    pub order_id: u64,
}

impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            symbol: "BTCUSDT".to_string(),
            order_id: 0,
        }
    }
}

#[derive(Clone)]
pub struct Accounts {
    pub binance: Option<binance::account::Account>,
    pub bitmex: Option<()>,
    pub huobi: Option<crate::api::huobi::Account>,
    pub stub: Option<()>,
}

impl Accounts {
    pub fn get(gateway_params: &GatewayParams) -> Self {
        let config_account = gateway_params.accounts.first().unwrap();

        let accounts = match gateway_params.exchange {
            ExchangeName::Binance => {
                let binance_account: binance::account::Account = Self::binance(&config_account);

                (Some(binance_account), None, None, None)
            }
            ExchangeName::BitMEX => {
                let bitmex_account = Self::bitmex(&config_account);

                (None, Some(bitmex_account), None, None)
            }

            ExchangeName::Huobi => {
                let huobi_account = Self::huobi(&config_account);

                (None, None, Some(huobi_account), None)
            }
            ExchangeName::StubExchange => (None, None, None, None),
        };
        Accounts {
            binance: accounts.0,
            bitmex: accounts.1,
            huobi: accounts.2,
            stub: accounts.3,
        }
    }

    pub fn binance(config_account: &GatewayParamsAccount) -> binance::account::Account {
        tokio::task::block_in_place(|| {
            Binance::new(
                Some(config_account.api_key.clone()),
                Some(config_account.secret_key.clone()),
            )
        })
    }

    pub fn huobi(config_account: &GatewayParamsAccount) -> crate::api::huobi::Account {
        crate::api::huobi::Account::new(
            config_account.account_id.as_ref().unwrap(),
            Some(config_account.api_key.clone()),
            Some(config_account.secret_key.clone()),
        )
    }

    fn bitmex(_config_account: &GatewayParamsAccount) -> () {}
}

impl Default for Accounts {
    fn default() -> Self {
        Accounts {
            binance: None,
            bitmex: None,
            huobi: None,
            stub: None,
        }
    }
}

pub struct WebSocket {
    pub huobi: Option<HuobiWS>,
}

impl WebSocket {
    pub fn get(gateway_params: &GatewayParams) -> Self {
        let instrument = gateway_params.instruments.first().unwrap();

        WebSocket {
            huobi: Some(HuobiWS::connect(&instrument.name)),
        }
    }
}

impl Default for WebSocket {
    fn default() -> Self {
        WebSocket {
            // huobi: Some(HuobiWS::connect("BTCUSDT")),
            huobi: None,
        }
    }
}

pub trait ExchangeAction: Sync + Send {
    fn inti(&self);

    fn fetch_depth(&self, symbol: &str) -> Result<Depth, &'static str>;

    fn fetch_balances(&self) -> Result<HashMap<String, HashMap<String, f64>>, &'static str>;

    fn limit_buy(&self, symbol: &str, amount: f64, price: f64) -> ExchangeApiResult<Transaction>;

    fn limit_sell(&self, symbol: &str, amount: f64, price: f64) -> ExchangeApiResult<Transaction>;

    fn market_buy(&self, symbol: &str, amount: f64) -> ExchangeApiResult<Transaction>;

    fn market_sell(&self, symbol: &str, amount: f64) -> ExchangeApiResult<Transaction>;

    fn cancel_order(&self, symbol: &str, order_id: u64) -> ExchangeApiResult<Transaction>;
}

// impl Exchange for binance::account::Account {
// }

#[cfg(test)]
mod tests {}
