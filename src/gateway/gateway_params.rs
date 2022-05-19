use super::GatewayConfig;
use crate::config::ParseConfig;
use std::{str::FromStr, string::ToString};
use strum_macros::{Display, EnumString};
use tracing::info;

#[derive(Debug, Clone)]
pub struct GatewayParamsAccount {
    pub name: String,
    // Some exchanges need account id for make request to them, e.g. Huobi
    pub account_id: Option<String>,
    pub api_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone)]
pub struct Instrument {
    pub name: String,
    pub base: String,
    pub quote: String,
    pub lot_size: f64,
    pub min_order_size: f64,
}

#[derive(Debug, Clone)]
pub struct Fee {
    pub account_name: String,
    pub amount_fee: f64,
}

#[derive(Debug, Clone)]
pub struct TimeLimit {
    pub rpc: u8,
}

impl Default for TimeLimit {
    fn default() -> Self {
        TimeLimit { rpc: 10 }
    }
}

#[derive(PartialEq, Debug, EnumString, Display, Clone)]
pub enum ExchangeName {
    Binance,
    Huobi,
    BitMEX,
    StubExchange,
}

#[derive(Debug, Clone)]
pub struct GatewayParams {
    pub name: String,
    pub exchange: ExchangeName,
    pub accounts: Vec<GatewayParamsAccount>,
    pub instruments: Vec<Instrument>,
    pub fees: Vec<Fee>,
    pub exchange_time_limit: TimeLimit,
}

pub trait GatewayParamsActions {
    fn from_config(config_file_path: &str) -> Result<GatewayParams, &'static str>;

    fn validate_config(gateway_config: &GatewayConfig) -> Result<(), &'static str>;
}

impl GatewayParamsActions for GatewayParams {
    fn from_config(config_file_path: &str) -> Result<GatewayParams, &'static str> {
        match GatewayConfig::from_file(config_file_path) {
            Ok(gateway_config) => match GatewayParams::validate_config(&gateway_config) {
                Ok(_) => Ok(GatewayParams::_from_config(gateway_config)),
                Err(_error) => Err("Config validation error"),
            },
            Err(_e) => Err("No gateway config"),
        }
    }

    // Validate Gateway config
    fn validate_config(_gateway_config: &GatewayConfig) -> Result<(), &'static str> {
        info!("Validating config for Gateway");
        Ok(())
    }
}

// Implement private methods
impl GatewayParams {
    fn _from_config(gateway_config: GatewayConfig) -> Self {
        GatewayParams {
            name: gateway_config.gateway_name,

            exchange: ExchangeName::from_str(&gateway_config.exchange).unwrap(),

            accounts: gateway_config
                .accounts
                .iter()
                .map(|a| GatewayParamsAccount {
                    name: a.name.clone(),
                    account_id: a.account_id.clone(),
                    api_key: a.api_key.clone(),
                    secret_key: a.secret_key.clone(),
                })
                .collect(),

            instruments: gateway_config
                .instruments
                .iter()
                .map(|i| Instrument {
                    name: i.name.clone(),
                    base: i.base.clone(),
                    quote: i.quote.clone(),
                    lot_size: i.lot_size,
                    min_order_size: i.min_order_size,
                })
                .collect(),

            fees: gateway_config
                .fees
                .iter()
                .map(|f| Fee {
                    account_name: f.account_name.clone(),
                    amount_fee: f.amount_fee,
                })
                .collect(),

            exchange_time_limit: TimeLimit {
                rpc: gateway_config.limit.rps,
            },
        }
    }
}

impl Default for GatewayParamsAccount {
    fn default() -> Self {
        GatewayParamsAccount {
            name: "StubAccount".to_string(),
            account_id: None,
            api_key: "API_KEY".to_string(),
            secret_key: "SECRE_KEY".to_string(),
        }
    }
}

impl Default for Fee {
    fn default() -> Self {
        Fee {
            account_name: "StubAccount".to_string(),
            amount_fee: 1.,
        }
    }
}

impl Default for GatewayParams {
    fn default() -> Self {
        GatewayParams {
            name: "DefaultStub".to_string(),
            exchange: ExchangeName::StubExchange,

            accounts: vec![GatewayParamsAccount::default()],
            instruments: vec![Instrument {
                name: "BTCUSDT".to_string(),
                base: "BTC".to_string(),
                quote: "USDT".to_string(),
                lot_size: 0.00001,
                min_order_size: 0.00001,
            }],
            fees: vec![Fee::default()],
            exchange_time_limit: TimeLimit::default(),
        }
    }
}

#[cfg(test)]
pub mod test_utils {

    use super::GatewayParamsAccount;
    pub struct GatewayParamsUtils;

    impl GatewayParamsUtils {
        pub fn binance_test_params() -> GatewayParamsAccount {
            let binance_api_key = env!("BINANCE_API_KEY").to_string();
            let binance_secret_key = env!("BINANCE_SECRET_KEY").to_string();

            GatewayParamsAccount {
                name: "BinanceTestAccount".to_string(),
                account_id: None,
                api_key: binance_api_key,
                secret_key: binance_secret_key,
            }
        }

        pub fn huobi_test_params() -> GatewayParamsAccount {
            let huobi_api_key = env!("HUOBI_API_KEY").to_string();
            let huobi_secret_key = env!("HUOBI_SECRET_KEY").to_string();
            let huobi_account_id = env!("HUOBI_ACCOUNT_ID").to_string();

            GatewayParamsAccount {
                name: "HuobiTestAccount".to_string(),
                account_id: Some(huobi_account_id),
                api_key: huobi_api_key,
                secret_key: huobi_secret_key,
            }
        }

        pub fn bitmex_test_params() -> GatewayParamsAccount {
            let bitmex_api_key = env!("BITMEX_API_KEY").to_string();
            let bitmex_secret_key = env!("BITMEX_SECRET_KEY").to_string();

            GatewayParamsAccount {
                name: "BitMexTestAccount".to_string(),
                account_id: None,
                api_key: bitmex_api_key,
                secret_key: bitmex_secret_key,
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn from_config() {
        let config_file_path = "test_files/gateway_config.toml";

        assert!(GatewayParams::from_config(config_file_path).is_ok());
    }

    #[test]
    fn validate_config() {
        let gateway_config = GatewayConfig::default();

        assert!(GatewayParams::validate_config(&gateway_config).is_ok());
    }
}
