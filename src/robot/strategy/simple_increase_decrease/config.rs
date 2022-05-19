use serde_derive::{Deserialize, Serialize};

use crate::config::ParseConfig;

#[derive(Debug, Deserialize, Serialize)]
pub struct Exchange {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SimpleIncreaseDecreaseStrategyConfig {
    pub name: String,
    pub description: String,
    pub instrument: String,
    pub increase_percentage: u8,
    pub decrease_percentage: u8,
    pub exchange: Exchange,
}

impl ParseConfig for SimpleIncreaseDecreaseStrategyConfig {}

impl Default for SimpleIncreaseDecreaseStrategyConfig {
    fn default() -> Self {
        Self {
            name: "IncreaseDecreaseBinance".to_string(),
            description:
                "Buy and sell instrument when its price changes by a certain amount of percent"
                    .to_string(),
            instrument: "BTCUSDT".to_string(),
            increase_percentage: 10,
            decrease_percentage: 10,
            exchange: Exchange {
                name: "Binance".to_string(),
            },
        }
    }
}

impl PartialEq for SimpleIncreaseDecreaseStrategyConfig {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.description == other.description
            && self.instrument == other.instrument
            && self.increase_percentage == other.increase_percentage
            && self.decrease_percentage == other.decrease_percentage
            && self.exchange == other.exchange
    }
}

impl PartialEq for Exchange {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[cfg(test)]
mod tests {
    use crate::config::get_config;

    use super::SimpleIncreaseDecreaseStrategyConfig;

    const TEST_SIMPLE_INCREASE_DECREASE_STRATEGY_CONFIG_FILE: &str =
        "test_files/simple_increase_decrease_strategy.toml";

    #[test]
    fn parse_config_file() {
        let simple_increase_decrease_strategy_config: SimpleIncreaseDecreaseStrategyConfig =
            get_config(TEST_SIMPLE_INCREASE_DECREASE_STRATEGY_CONFIG_FILE).unwrap();

        assert_eq!(
            simple_increase_decrease_strategy_config,
            SimpleIncreaseDecreaseStrategyConfig::default()
        );
    }
}
