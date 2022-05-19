use crate::config::ParseConfig;
use serde_derive::{Deserialize, Serialize};
use std::cmp::PartialEq;

pub const GATEWAY_CONFIG_FILE: &str = "conf/gateway_config.toml";

#[derive(Deserialize, Debug, Serialize)]
pub struct GatewayConfig {
    pub gateway_name: String,
    pub exchange: String,
    pub accounts: Vec<Account>,
    pub instruments: Vec<Instrument>,
    pub fees: Vec<Fee>,
    pub limit: Limit,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Account {
    pub name: String,
    pub account_id: Option<String>,
    pub api_key: String,
    pub secret_key: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Instrument {
    pub name: String,
    pub base: String,
    pub quote: String,
    pub lot_size: f64,
    pub min_order_size: f64,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Fee {
    pub account_name: String,
    pub amount_fee: f64,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Limit {
    pub rps: u8,
}

impl ParseConfig for GatewayConfig {}

impl Default for GatewayConfig {
    fn default() -> Self {
        GatewayConfig {
            gateway_name: "Huobi".to_string(),

            exchange: "Huobi".to_string(),

            accounts: vec![
                Account {
                    name: "Account1".to_string(),
                    account_id: Some("12345".to_string()),
                    api_key: "API_KEY1".to_string(),
                    secret_key: "SECRET_KEY1".to_string(),
                },
                Account {
                    name: "Account2".to_string(),
                    account_id: Some("67890".to_string()),
                    api_key: "API_KEY2".to_string(),
                    secret_key: "SECRET_KEY2".to_string(),
                },
            ],

            instruments: vec![
                Instrument {
                    name: "BTCUSDT".to_string(),
                    base: "BTC".to_string(),
                    quote: "USDT".to_string(),
                    lot_size: 0.00001,
                    min_order_size: 0.00001,
                },
                Instrument {
                    name: "ETHUSDT".to_string(),
                    base: "ETH".to_string(),
                    quote: "USDT".to_string(),
                    lot_size: 0.01,
                    min_order_size: 0.01,
                },
            ],

            fees: vec![
                Fee {
                    account_name: "Account1".to_string(),
                    amount_fee: 2.5,
                },
                Fee {
                    account_name: "Account2".to_string(),
                    amount_fee: 3.5,
                },
            ],

            limit: Limit { rps: 10 },
        }
    }
}

impl PartialEq for Account {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.api_key == other.api_key
            && self.secret_key == other.secret_key
    }
}

impl PartialEq for Instrument {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.base == other.base
            && self.quote == other.quote
            && self.lot_size == other.lot_size
            && self.min_order_size == other.min_order_size
    }
}

impl PartialEq for Fee {
    fn eq(&self, other: &Self) -> bool {
        self.account_name == other.account_name && self.amount_fee == other.amount_fee
    }
}

impl PartialEq for Limit {
    fn eq(&self, other: &Self) -> bool {
        self.rps == other.rps
    }
}

impl PartialEq for GatewayConfig {
    fn eq(&self, other: &Self) -> bool {
        self.gateway_name == other.gateway_name
            && self.exchange == other.exchange
            && self.accounts == other.accounts
            && self.instruments == other.instruments
            && self.fees == other.fees
            && self.limit == other.limit
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::get_config;

    const TEST_GATEWAY_CONFIG_FILE: &str = "test_files/gateway_config.toml";

    #[test]
    fn parse_config_file() {
        let gateway_config: GatewayConfig = get_config(TEST_GATEWAY_CONFIG_FILE).unwrap();

        assert_eq!(gateway_config, GatewayConfig::default());
    }
}
