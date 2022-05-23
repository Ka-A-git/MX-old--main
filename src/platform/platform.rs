use super::metrics::Metrics;
use super::utils::uppercase_first_letter; 
use super::{Environment, PlatformConfig, PLATFORM_CONFIG_FILE_PATH};
use crate::{
    context_manager::{ContextManager, ContextMsg, GatewayMsg},  
    gateway::{GatewayEnvironment, GatewayStatus},
    order_manager::{ActiveOrderMsg, OrderManager, OrderMsg},
    robot::{RobotEnvironment, RobotStatus},
    storage::{sensors::SensorManager, SensorMsg},
};
use crossbeam::channel::unbounded;
use crossbeam::channel::{Receiver, Sender};
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::Duration;
use std::{collections::HashMap, sync::RwLock};
use strum_macros::{Display, EnumString};
use tracing::{debug, info}; 

lazy_static! {
    pub static ref ROBOT_TO_GATEWAY_TIMES: Mutex<Vec<Duration>> = Mutex::new(Vec::new());
    pub static ref GATEWAY_TO_ROBOT_TIMES: Mutex<Vec<Duration>> = Mutex::new(Vec::new());
}

#[derive(Debug, EnumString, Display, PartialEq)] 
pub enum PlatformStatus {
    Active,
    Stopped,
}

pub struct Platform {
    pub context_manager: ContextManager,

    pub order_manager: OrderManager,

    // Environment: Robots and Gateways
    pub environment: Environment,
    pub sensor_manager: SensorManager,

    // Current status of Platform: Active|Stopped
    status: RwLock<PlatformStatus>,
}

pub enum Status {
    Working,
    Stopping, 
}

impl<'a> Platform {
    // Initialize platform with default configure file
    pub fn init() -> Self {
        Platform::internal_init(PLATFORM_CONFIG_FILE_PATH)
    }

    // Initialize platform and all its environment with a configure file
    fn internal_init(platform_config_file_path: &str) -> Self {
        info!("Initializing Platform");

        let platform_config = PlatformConfig::get_config(platform_config_file_path);

        //--- Initial state

        // Getting Info for Context Manager from Gateway
        // Gateways fetch depth information from exchanges
        // Sender: Gateways
        // Receiver: Context Manager
        let (info_sender_from_gateway, info_receiver_to_contextmanager): (
            Sender<GatewayMsg>,
            Receiver<GatewayMsg>,
        ) = unbounded();

        // Sending Orders from Robots to Order Manager
        // Sender: Robots
        // Receiver: Order Manager
        let (order_sender_from_robot, order_receiver_to_ordermanager): (
            Sender<OrderMsg>,
            Receiver<OrderMsg>,
        ) = unbounded();
 
        // Sending Orders from Order Manager to Gateways
        // Sender: Order Manager
        // Receiver: Gateways
        // Get channels for Gateways
        let (order_senders_from_ordermanager, order_receivers_to_gateway) =
            PlatformUtils::get_gateway_channels(&platform_config);

        // Gateways sends active and filled orders on exchange to Order Manager
        // Sender: Gateways
        // Receiver: Order Manager
        let (active_order_sender, active_order_receiver): (
            Sender<ActiveOrderMsg>,
            Receiver<ActiveOrderMsg>,
        ) = unbounded();

        // Get channels for Robots
        // Context Manager sends info (OrderBooks and positions) messages to Robots
        // Sender: Context Manager
        // Receiver: Robots
        let (info_senders_from_context_manager, info_receivers_to_robot) =
            PlatformUtils::get_robot_channels(&platform_config);

        let (sensor_sender, sensor_receiver): (Sender<SensorMsg>, Receiver<SensorMsg>) =
            unbounded();

        // Load robots
        let robot_environment = RobotEnvironment::load_robot_environment(
            platform_config.robots.clone(),
            info_receivers_to_robot,
            order_sender_from_robot,
            sensor_sender.clone(),
        )
        .unwrap();

        info!("[Platform] Loaded {} robots", platform_config.robots.len());
        debug!("Robots are loaded: {:?}", platform_config.robots);

        // Load gateways
        let gateway_environment = GatewayEnvironment::load_gateway_environment(
            platform_config.robots.clone(),
            platform_config.gateways.clone(),
            info_sender_from_gateway.clone(),
            order_receivers_to_gateway,
            active_order_sender,
        )
        .unwrap();

        info!(
            "[Platform] Loaded {} gateways",
            platform_config.gateways.len()
        );
        debug!("Gateways are loaded: {:?}", platform_config.gateways);

        let host_address = platform_config.influxdb.host_address.clone();

        Platform {
            status: RwLock::new(PlatformStatus::Stopped),

            context_manager: ContextManager::init(
                info_senders_from_context_manager,
                info_receiver_to_contextmanager,
                platform_config
                    .gateways
                    .iter()
                    .map(|g| g.name.clone())
                    .collect(),
                PlatformUtils::get_robot_subscriptions(&platform_config),
            ),

            order_manager: OrderManager::init(
                order_senders_from_ordermanager,
                order_receiver_to_ordermanager.clone(),
                active_order_receiver,
            ),

            sensor_manager: SensorManager::new(sensor_receiver, host_address),

            environment: Environment::init(robot_environment, gateway_environment),
        }
    }

    pub fn start(&'static self) -> Result<(), &'static str> {
        info!("Starting Platform");
        {
            let status_lock = self.status.read().unwrap();
            if *status_lock == PlatformStatus::Active {
                return Err("Platform is already running");
            }
        }
        {
            let mut status_lock = self.status.write().unwrap();
            *status_lock = PlatformStatus::Active;
        }

        self.environment.robot_environment.start_all_robots()?;

        self.environment.gateway_environment.start_all_gateways()?;

        self.context_manager.start()?;

        self.order_manager.start()?;

        self.sensor_manager.start()?;

        Ok(())
    }

    // Stop platform and all its processes
    pub fn stop(&self) -> Result<(), &'static str> {
        info!("Stopping Platform");
        {
            let status_lock = self.status.read().unwrap();
            if *status_lock == PlatformStatus::Stopped {
                return Err("Platform is already stopped");
            }
        }

        match self.environment.robot_environment.robots.read() {
            Ok(robots) => {
                for robot in &*robots {
                    if robot.status().unwrap() == RobotStatus::Active {
                        robot.stop().unwrap();
                    }
                }
            }
            Err(_) => todo!(),
        }
        self.context_manager.stop()?;

        self.order_manager.stop()?;

        for gateway in &*self
            .environment
            .gateway_environment
            .gateways
            .read()
            .unwrap()
        {
            if gateway.status().unwrap() == GatewayStatus::Active {
                gateway.stop()?;
            }
        } 

        {
            let mut status_lock = self.status.write().unwrap();
            *status_lock = PlatformStatus::Stopped;
        }

        let robot_to_gateway =
            Metrics::init(ROBOT_TO_GATEWAY_TIMES.lock().unwrap().clone()).unwrap();

        println!("Robot to Gateway{}", robot_to_gateway.calc());

        let gateway_to_robot =
            Metrics::init(GATEWAY_TO_ROBOT_TIMES.lock().unwrap().clone()).unwrap();

        println!("");

        println!("Gateway to Robot {}", gateway_to_robot.calc());

        Ok(())
    }

    // Get status of Platform
    pub fn status(&self) -> Result<String, &'static str> {
        info!("Getting status of Platform");
        match self.status.read() {
            Ok(status) => Ok(format!("{}", *status)),
            Err(_lock_error) => Err("Platform status lock error"),
        }
    }

    // Get information about platform and its robots and gateways
    pub fn info(&self) -> Result<Vec<String>, &'static str> {
        info!("Getting info about Platform");

        let mut info = vec![];

        info.push("Robots:\n".to_string());
        for robot in &*self.environment.robot_environment.robots.read().unwrap() {
            info.push(robot.info()?);
        }

        info.push("Gateways:\n".to_string());
        for gateway in &*self
            .environment
            .gateway_environment
            .gateways
            .read()
            .unwrap()
        {
            info.push(gateway.info()?);
        }

        Ok(info)
    }
}

pub struct PlatformUtils;

impl PlatformUtils {
    pub fn get_robot_channels(
        platform_config: &PlatformConfig,
    ) -> (
        HashMap<String, Sender<ContextMsg>>,
        HashMap<String, Receiver<ContextMsg>>,
    ) {
        fn robot_channels(
            robot_names: Vec<String>,
        ) -> (
            HashMap<String, Sender<ContextMsg>>,
            HashMap<String, Receiver<ContextMsg>>,
        ) {
            let mut senders = HashMap::new();
            let mut receivers = HashMap::new();

            for robot_name in robot_names {
                let (sender, receiver) = unbounded();
                senders.insert(robot_name.clone(), sender);
                receivers.insert(robot_name.clone(), receiver);
            }

            (senders, receivers)
        }

        let (robot_senders, robot_receivers) = robot_channels(
            platform_config
                .robots
                .clone()
                .iter()
                .map(|robot| uppercase_first_letter(robot.name.clone()))
                .collect(),
        );

        (robot_senders, robot_receivers)
    }

    pub fn get_gateway_channels(
        platform_config: &PlatformConfig,
    ) -> (
        HashMap<String, Sender<OrderMsg>>,
        HashMap<String, Receiver<OrderMsg>>,
    ) {
        fn gateway_channels(
            gateway_names: Vec<String>,
        ) -> (
            HashMap<String, Sender<OrderMsg>>,
            HashMap<String, Receiver<OrderMsg>>,
        ) {
            let mut senders = HashMap::new();
            let mut receivers = HashMap::new();

            for gateway_name in gateway_names {
                let (sender, receiver) = unbounded();
                senders.insert(gateway_name.clone(), sender);
                receivers.insert(gateway_name.clone(), receiver);
            }

            (senders, receivers)
        }

        let (gateway_senders, gateway_receivers) = gateway_channels(
            platform_config
                .gateways
                .iter()
                .map(|gateway| uppercase_first_letter(gateway.name.clone()))
                .collect(),
        );

        (gateway_senders, gateway_receivers)
    }

    // <RobotID, <Gateway, [Symbols]>>
    pub fn get_robot_subscriptions(
        _platform_config: &PlatformConfig,
    ) -> HashMap<String, HashMap<String, Vec<String>>> {
        let subscriptions = HashMap::new();

        subscriptions
    }
}

#[cfg(test)]
mod tests {

    use super::{Platform, PlatformStatus};

    const TEST_CONFIG_FILE_PATH: &str = "test_files/platform_config.toml";

    fn get_platform_stub() -> &'static Platform {
        Box::leak(Box::new(Platform::internal_init(TEST_CONFIG_FILE_PATH)))
    }

    #[tokio::test]
    async fn start_platform() {
        let platform = get_platform_stub();

        assert!(platform.status().unwrap() == format!("{}", PlatformStatus::Stopped));
        assert!(platform.start().is_ok());
    }

    #[tokio::test]
    async fn stop_platform() {
        let platform = get_platform_stub();

        assert!(platform.status().unwrap() == format!("{}", PlatformStatus::Stopped));
        assert!(platform.start().is_ok());
        assert!(platform.status().unwrap() == format!("{}", PlatformStatus::Active));
        assert!(platform.stop().is_ok());
        assert!(platform.status().unwrap() == format!("{}", PlatformStatus::Stopped));
    }

    #[tokio::test]
    async fn start_platform_twice() {
        let platform = get_platform_stub();

        assert!(platform.start().is_ok());
        assert!(platform.status().unwrap() == format!("{}", PlatformStatus::Active));
        assert!(platform.start().is_err());
    }

    #[tokio::test]
    async fn stop_platform_twice() {
        let platform = get_platform_stub();

        assert!(platform.status().unwrap() == format!("{}", PlatformStatus::Stopped));
        assert!(platform.start().is_ok());
        assert!(platform.status().unwrap() == format!("{}", PlatformStatus::Active));
        assert!(platform.stop().is_ok());
        assert!(platform.status().unwrap() == format!("{}", PlatformStatus::Stopped));
        assert!(platform.stop().is_err());
    }

    #[test]
    fn stop_platform_without_start() {
        let platform = Platform::internal_init(TEST_CONFIG_FILE_PATH);
        assert!(platform.stop().is_err());
    }

    #[test]
    fn check_duplicate_robot_names() {
        let _platform = Platform::internal_init(TEST_CONFIG_FILE_PATH);
        // TODO
    }

    #[test]
    fn check_duplicate_gateway_names() {
        let _platform = Platform::internal_init(TEST_CONFIG_FILE_PATH);
        // TODO
    }

    #[test]
    fn status() {
        let platform = Platform::internal_init(TEST_CONFIG_FILE_PATH);
        assert!(platform.status().is_ok());
    }

    #[test]
    #[ignore]
    // For local testing
    fn status_print() {
        let platform = Platform::internal_init(TEST_CONFIG_FILE_PATH);
        println!("Platform status: {:?}", platform.status());
    }

    #[test]
    fn info() {
        let platform = Platform::internal_init(TEST_CONFIG_FILE_PATH);
        assert!(platform.info().is_ok());
    }

    #[test]
    #[ignore]
    // For local testing
    fn info_print() {
        let platform = Platform::internal_init(TEST_CONFIG_FILE_PATH);
        println!("Platform info: {:?}", platform.info().unwrap());
    }
}
