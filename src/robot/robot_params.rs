use super::config::RobotConfig;
use super::strategy::{SimpleIncreaseDecreaseStrategy, Strategy};
use crate::config::ParseConfig;
use std::{cmp::PartialEq, fmt::Debug, str::FromStr, sync::RwLock};
use strum_macros::{Display, EnumString};
use tracing::info;

#[derive(PartialEq, Debug, EnumString, Display, Clone)]
pub enum RobotStrategyType {
    Arbitration,
    SimpleIncreaseDecrease,
    Demo,
    Prod,
}

#[derive(PartialEq, Debug, EnumString, Display, Clone)]
pub enum RobotGateways {
    Binance,
    Bitstamp,
    Bittrex,
    Coinbase,
    Huobi,
    Kraken,
    Simplex,
}

#[derive(Debug)]
pub struct RobotParams {
    pub name: String,
    pub strategy_type: RobotStrategyType,
    pub strategy: Box<dyn Strategy>,
    pub gateway: RobotGateways,
    pub pnl: RobotPNL,
}

#[derive(Debug, Clone)]
pub struct RobotPNL {
    pub components: Vec<PNLComponent>,
    pub currency: String,
    pub max_loss: i32,
    pub stop_loss: i32,
}

#[derive(Debug, Clone)]
pub struct PNLComponent {
    pub instrument: String,
    pub gateway: String,
    pub bad_deal_chain_sequence: bool,
    pub price_hint: String,
}

pub trait RobotParamsActions {
    fn from_config(config_file_path: &str) -> Result<RobotParams, &'static str>;

    fn validate_config(robot_config: &RobotConfig) -> Result<(), &'static str>;

    // fn set_config(&self, config_file_path: &str) -> Result<(), &'static str>;
    // fn set_config(
    //     &self,
    //     robot_params: RwLock<RobotParams>,
    //     file_config_path: &str,
    // ) -> Result<(), &'static str>;
}

impl RobotParamsActions for RobotParams {
    fn from_config(config_file_path: &str) -> Result<Self, &'static str> {
        match RobotConfig::from_file(config_file_path) {
            Ok(robot_config) => match RobotParams::validate_config(&robot_config) {
                Ok(_) => Ok(RobotParams::_from_config(robot_config)),
                Err(_error) => Err("Config validation error"),
            },
            Err(_e) => Err("No robot config"),
        }
    }

    fn validate_config(_robot_config: &RobotConfig) -> Result<(), &'static str> {
        info!("Validating config for Robot");
        // TODO
        Ok(())
    }
}

impl RobotParams {
    fn _from_config(robot_config: RobotConfig) -> Self {
        RobotParams {
            name: robot_config.name,
            strategy_type: match robot_config.strategy.strategy_type {
                super::config::RobotConfigStrategyType::Arbitration => {
                    RobotStrategyType::Arbitration
                }
                super::config::RobotConfigStrategyType::SimpleIncreaseDecrease => {
                    RobotStrategyType::SimpleIncreaseDecrease
                }
            },
            strategy: match robot_config.strategy.strategy_type {
                super::config::RobotConfigStrategyType::Arbitration => Box::new(
                    ArbitrationStrategy::from_config(&robot_config.strategy.config_file_path)
                        .unwrap(),
                ),
                super::config::RobotConfigStrategyType::SimpleIncreaseDecrease => Box::new(
                    SimpleIncreaseDecreaseStrategy::from_config(
                        &robot_config.strategy.config_file_path,
                    )
                    .unwrap(),
                ),
            },
            gateway: RobotGateways::from_str(&robot_config.gateway).unwrap(),
            pnl: RobotPNL {
                components: robot_config
                    .pnl
                    .components
                    .iter()
                    .map(|c| PNLComponent {
                        instrument: c.instrument.clone(),
                        gateway: c.gateway.clone(),
                        bad_deal_chain_sequence: c.bad_deal_chain_sequence,
                        price_hint: c.price_hint.clone(),
                    })
                    .collect(),
                currency: robot_config.pnl.currency,
                max_loss: robot_config.pnl.max_loss,
                stop_loss: robot_config.pnl.stop_loss,
            },
        }
    }

    fn _set_config(robot_params: RwLock<RobotParams>, robot_config: RobotConfig) {
        let mut robot_params_lock = robot_params.write().unwrap();

        robot_params_lock.name = robot_config.name;

        robot_params_lock.strategy_type = match robot_config.strategy.strategy_type {
            super::config::RobotConfigStrategyType::Arbitration => RobotStrategyType::Arbitration,
            super::config::RobotConfigStrategyType::SimpleIncreaseDecrease => {
                RobotStrategyType::SimpleIncreaseDecrease
            }
        };

        robot_params_lock.strategy = match robot_config.strategy.strategy_type {
            super::config::RobotConfigStrategyType::Arbitration => Box::new(
                ArbitrationStrategy::from_config(&robot_config.strategy.config_file_path).unwrap(),
            ),
            super::config::RobotConfigStrategyType::SimpleIncreaseDecrease => {
                Box::new(SimpleIncreaseDecreaseStrategy::default()) // TODO replace with new()
            }
        };

        robot_params_lock.gateway = RobotGateways::from_str(&robot_config.gateway).unwrap();
        robot_params_lock.pnl = RobotPNL {
            components: robot_config
                .pnl
                .components
                .iter()
                .map(|c| PNLComponent {
                    instrument: c.instrument.clone(),
                    gateway: c.gateway.clone(),
                    bad_deal_chain_sequence: c.bad_deal_chain_sequence,
                    price_hint: c.price_hint.clone(),
                })
                .collect(),
            currency: robot_config.pnl.currency,
            max_loss: robot_config.pnl.max_loss,
            stop_loss: robot_config.pnl.stop_loss,
        };
    }
}

impl Default for RobotPNL {
    fn default() -> Self {
        RobotPNL {
            components: vec![PNLComponent {
                instrument: "BTCUSDT".to_string(),
                gateway: "Huobi::PROD".to_string(),
                bad_deal_chain_sequence: true,
                price_hint: "BOOK(BTCUSDT, Huobi::PROD, 1)".to_string(),
            }],
            currency: "USDT".to_string(),
            max_loss: 10,
            stop_loss: 50,
        }
    }
}

impl Default for RobotParams {
    fn default() -> Self {
        RobotParams {
            name: "Robot".to_string(),
            strategy_type: RobotStrategyType::Demo,
            strategy: Box::new(ArbitrationStrategy::default()),
            gateway: RobotGateways::Huobi,
            pnl: RobotPNL::default(),
        }
    }
}

impl PartialEq for RobotParams {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.strategy_type == other.strategy_type
            // && self.strategy == other.strategy
            && self.gateway == other.gateway
            // && *self.status.read().unwrap() == *other.status.read().unwrap()
            && self.pnl == other.pnl
    }
}

impl PartialEq for RobotPNL {
    fn eq(&self, other: &Self) -> bool {
        self.components == other.components
            && self.currency == other.currency
            && self.max_loss == other.max_loss
            && self.stop_loss == other.stop_loss
    }
}

impl PartialEq for PNLComponent {
    fn eq(&self, other: &Self) -> bool {
        self.instrument == other.instrument
            && self.gateway == other.gateway
            && self.bad_deal_chain_sequence == other.bad_deal_chain_sequence
            && self.price_hint == other.price_hint
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn from_config() {
        let config_file_path = "test_files/robot_config.toml";

        assert!(RobotParams::from_config(config_file_path).is_ok());
    }

    #[test]
    fn validate_config() {
        let robot_config = RobotConfig::default();

        assert!(RobotParams::validate_config(&robot_config).is_ok());
    }
}
