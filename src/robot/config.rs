use crate::config::ParseConfig;
use serde_derive::{Deserialize, Serialize};
use std::cmp::PartialEq;

pub const ROBOT_CONFIG_FILE: &str = "conf/robot_config.toml";

#[derive(Deserialize, Serialize, Debug)]
pub struct RobotConfig {
    pub name: String,
    pub gateway: String,
    pub strategy: RobotConfigStrategy,
    pub pnl: RobotConfigPNL,
}
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum RobotConfigStrategyType {
    Arbitration,
    SimpleIncreaseDecrease,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RobotConfigStrategy {
    pub name: String,
    pub strategy_type: RobotConfigStrategyType,
    pub config_file_path: String,
}

impl Default for RobotConfig {
    fn default() -> Self {
        RobotConfig {
            name: "Robot_Huobi_Demo".to_string(),
            gateway: "Huobi".to_string(),
            strategy: RobotConfigStrategy::default(),
            // instruments: vec![
            //     RobotInstrument {
            //         name: "BTCUSDT".to_string(),
            //     },
            //     RobotInstrument {
            //         name: "ETHUSDT".to_string(),
            //     },
            // ],
            pnl: RobotConfigPNL::default(),
        }
    }
}

impl Default for RobotConfigStrategy {
    fn default() -> Self {
        RobotConfigStrategy {
            name: "ArbitrationStrategy".to_string(),
            strategy_type: RobotConfigStrategyType::Arbitration,
            config_file_path: "test_files/arbitration_strategy.toml".to_string(),
        }
    }
}
impl PartialEq for RobotConfigStrategy {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.strategy_type == other.strategy_type
            && self.config_file_path == other.config_file_path
    }
}

impl PartialEq for RobotConfig {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.gateway == other.gateway
            && self.strategy == other.strategy
            // && self.instruments == other.instruments
            && self.pnl == other.pnl
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RobotInstrument {
    name: String,
}

impl Default for RobotInstrument {
    fn default() -> Self {
        RobotInstrument {
            name: "BTCUSDT".to_string(),
        }
    }
}

impl PartialEq for RobotInstrument {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RobotConfigPNL {
    pub currency: String,
    pub max_loss: i32,
    pub stop_loss: i32,
    pub components: Vec<PNLComponent>,
}

impl Default for RobotConfigPNL {
    fn default() -> Self {
        RobotConfigPNL {
            components: vec![PNLComponent::default(), PNLComponent::default()],
            currency: "USDT".to_string(),
            max_loss: 10,
            stop_loss: 0,
        }
    }
}

impl PartialEq for RobotConfigPNL {
    fn eq(&self, other: &Self) -> bool {
        self.components == other.components
            && self.currency == other.currency
            && self.max_loss == other.max_loss
            && self.stop_loss == other.stop_loss
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PNLComponent {
    pub instrument: String,
    pub gateway: String,
    pub bad_deal_chain_sequence: bool,
    pub price_hint: String,
}

impl Default for PNLComponent {
    fn default() -> Self {
        PNLComponent {
            instrument: "BTCUSDT".to_string(),
            gateway: "Huobi::PROD".to_string(),
            bad_deal_chain_sequence: true,
            price_hint: "BOOK(BTCUSDT, Huobi::PROD, 1)".to_string(),
        }
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

impl ParseConfig for RobotConfig {}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::get_config;

    const TEST_ROBOT_CONFIG_FILE: &str = "test_files/robot_config.toml";

    #[test]
    fn test_parse_file() {
        let robot_config: RobotConfig = get_config(TEST_ROBOT_CONFIG_FILE).unwrap();
        assert_eq!(robot_config, RobotConfig::default());
    }
}
