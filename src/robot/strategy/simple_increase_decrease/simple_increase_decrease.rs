use crate::{
    config::ParseConfig,
    context_manager::{ContextInfo, GatewayMsg},
    robot::strategy::{Action, Strategy},
    storage::sensors::InfluxPoint,
    SimpleIncreaseDecreaseStrategyConfig,
};
use std::sync::RwLock;
use tracing::info;

#[derive(Debug)]
pub struct SimpleIncreaseDecreaseStrategy {
    name: String,
    description: String,
    instrument: String,
    initial_time: u64,
    interval: u32,
    initial_price: f64,
    current_time: u64,
    increase_percentage: u8,
    decrease_percentage: u8,
    data: RwLock<Vec<GatewayMsg>>,
}

impl Strategy for SimpleIncreaseDecreaseStrategy {
    fn start(&self) -> Result<(), &'static str> {
        info!("Starting SID Strategy");
        Ok(())
    }

    fn get_data(&self) -> Result<ContextInfo, &'static str> {
        Ok(ContextInfo::default())
    }

    fn load_data(&self, _context_info: ContextInfo) -> Result<(), &'static str> {
        info!("Loading data for SID Strategy");
        Ok(())
    }

    fn calc(&self) -> Result<(Vec<Action>, Vec<InfluxPoint>), &'static str> {
        info!("Calculating SID Strategy");
        Ok((vec![Action::default()], Vec::new()))
    }

    fn buy(&self) -> bool {
        todo!()
    }

    fn sell(&self) -> bool {
        todo!()
    }

    fn clear_data(&self) -> Result<(), &'static str> {
        match self.data.write() {
            Ok(mut data) => {
                data.clear();
                Ok(())
            }
            Err(_e) => Err("Lock error"),
        }
    }

    fn finish(&self) -> Result<(), &'static str> {
        info!("Finishing SID Strategy");

        self.clear_data()
    }
}

impl SimpleIncreaseDecreaseStrategy {
    pub fn new(name: &str, description: &str, instrument: &str) -> Self {
        SimpleIncreaseDecreaseStrategy {
            name: name.to_string(),
            description: description.to_string(),
            instrument: instrument.to_string(),
            initial_time: 0,
            interval: 0,
            initial_price: 0.,
            current_time: 0,
            increase_percentage: 10,
            decrease_percentage: 10,
            data: RwLock::new(Vec::new()),
        }
    }

    pub fn from_config(
        config_file_path: &str,
    ) -> Result<SimpleIncreaseDecreaseStrategy, &'static str> {
        match SimpleIncreaseDecreaseStrategyConfig::from_file::<SimpleIncreaseDecreaseStrategyConfig>(
            config_file_path,
        ) {
            Ok(strategy_config) => {
                Ok(SimpleIncreaseDecreaseStrategy {
                    name: strategy_config.name,
                    description: strategy_config.description,
                    instrument: strategy_config.instrument,
                    initial_time: 0,
                    interval: 0, // TODO  do we need it?
                    initial_price: 0.,
                    current_time: 0,
                    increase_percentage: strategy_config.increase_percentage,
                    decrease_percentage: strategy_config.decrease_percentage,
                    data: RwLock::new(Vec::new()),
                    // do we need to add an exchange field?
                })
            }
            Err(_e) => Err("No strategy"),
        }
    }
}

impl Default for SimpleIncreaseDecreaseStrategy {
    fn default() -> Self {
        SimpleIncreaseDecreaseStrategy {
            name: "IncreaseDecreaseBinance".to_string(),
            description:
                "Buy and sell instruemnt when its price changes by a certain amount of percent"
                    .to_string(),
            instrument: "BTCUSDT".to_string(),
            initial_time: 0,
            interval: 0,
            initial_price: 0.,
            current_time: 0,
            increase_percentage: 10,
            decrease_percentage: 10,
            data: RwLock::new(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::SimpleIncreaseDecreaseStrategy;
    use super::Strategy;
    use crate::context_manager::ContextInfo;

    #[test]
    fn start() {
        let sid_strategy = SimpleIncreaseDecreaseStrategy::default();

        assert!(sid_strategy.start().is_ok());
    }

    #[test]
    fn load_data() {
        let sid_strategy = SimpleIncreaseDecreaseStrategy::default();

        let context_info = ContextInfo::default();

        assert!(sid_strategy.load_data(context_info).is_ok());
    }

    #[test]
    fn calc() {
        let sid_strategy = SimpleIncreaseDecreaseStrategy::default();

        assert!(sid_strategy.calc().is_ok());
    }

    #[test]
    #[ignore]
    fn buy() {}

    #[test]
    #[ignore]
    fn sell() {}

    #[test]
    fn finish() {
        let sid_strategy = SimpleIncreaseDecreaseStrategy::default();

        assert!(sid_strategy.finish().is_ok());
    }

    #[test]
    fn from_config() {
        let strategy_config_file_path = "test_files/simple_increase_decrease_strategy.toml";

        assert!(SimpleIncreaseDecreaseStrategy::from_config(strategy_config_file_path).is_ok());
    }
}
