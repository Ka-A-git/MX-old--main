use super::config::RobotConfig;
use super::risk_control::{RiskControl, RiskLimit, BAD_DEAL_TIME, NUMBER_OF_BAD_DEALS};
use super::strategy::{ArbitrationStrategy, OrderType, SimpleIncreaseDecreaseStrategy};
use super::{
    PNLComponent, RobotGateways, RobotPNL, RobotParams, RobotParamsActions, RobotStrategyType,
};
use crate::context_manager::{ContextInfo, ContextMsg};
use crate::order_manager::{LimitOrder, MarketOrder, Order, OrderContainer, OrderMsg};
use crate::platform::GATEWAY_TO_ROBOT_TIMES;
use crate::storage::SensorMsg;
use crate::{config::ParseConfig, storage::sensors::InfluxPoint};
use chrono::Utc;
use crossbeam::channel::{bounded, unbounded, Receiver, Sender};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Instant;
use std::{cmp::PartialEq, fmt::Debug, str::FromStr, sync::RwLock, thread, thread::JoinHandle};
use strum_macros::Display;
use tracing::{debug, error, info};

#[derive(Debug, PartialEq, Clone, Display)]
pub enum RobotStatus {
    Active,
    Stopped,
    Locked,
}

#[derive(Debug)]
pub struct Robot {
    pub robot_params: RwLock<RobotParams>,

    risk_control: RwLock<RiskControl>,

    // Receives context info(Market data and positions) from different gateways
    context_info_receiver: Receiver<ContextMsg>,

    pub order_sender: Sender<OrderMsg>,
    pub sensor_sender: Sender<SensorMsg>,

    pub(self) stop_channel: (Sender<()>, Receiver<()>),

    // Stores context info(Market data and positions)
    context_info_store: RwLock<ContextInfo>,

    status: RwLock<RobotStatus>,
}

impl PartialEq for Robot {
    fn eq(&self, other: &Self) -> bool {
        *self.robot_params.read().unwrap() == *other.robot_params.read().unwrap()
    }
}

impl Robot {
    // Starts Robot and all its dependent threads
    pub fn start(&'static self) -> Result<JoinHandle<()>, &'static str> {
        info!("Starting {} Robot", self.robot_params.read().unwrap().name);

        match self.status.write() {
            Ok(mut status) => match *status {
                RobotStatus::Active => return Err("Robot is already running"),
                RobotStatus::Stopped => {
                    *status = RobotStatus::Active;

                    info!(
                        "Robot {} has been started",
                        self.robot_params.read().unwrap().name
                    );

                    // Starts strategy
                    self.robot_params.read().unwrap().strategy.start()?;

                    // Runs thread for Robot's main cycle
                    let handle = thread::spawn(move || {
                        info!(
                            "[Robot] Starting {} Robot's main cycle",
                            self.robot_params.read().unwrap().name
                        );

                        loop {
                            self.main_cycle().unwrap();
                            // self.on_post_cycle();
                            if self.stop_channel.1.try_recv().is_ok() {
                                break;
                            }
                        }
                    });

                    // Runs thread for receiving info for Robot
                    let _ = thread::spawn(move || {
                        info!(
                            "[Robot] Robot {} is starting  receiving info",
                            self.robot_params.read().unwrap().name
                        );

                        loop {
                            match self.receive_info() {
                                Ok(status) => {
                                    if let RobotStatus::Locked = status {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!(e)
                                }
                            }

                            match self.stop_channel.1.try_recv() {
                                Ok(_) => break,
                                _ => {}
                            }
                        }
                    });

                    Ok(handle)
                }
                RobotStatus::Locked => Err("Robot is locked"),
            },
            Err(_lock_error) => Err("Lock error"),
        }
    }

    fn send_stop(&self) -> Result<(), &'static str> {
        match self.stop_channel.0.send(()) {
            Ok(_) => {
                self.stop_channel.0.try_send(()).unwrap();

                info!(
                    "Stop signal was sent to {} Robot. Robot's main cycle was stopped",
                    self.robot_params.read().unwrap().name
                );
                Ok(())
            }
            Err(_channel_error) => Err("Robot hasn't stopped, channel error"),
        }
    }

    // Stops Robot and all its dependent threads
    pub fn stop(&self) -> Result<(), &'static str> {
        info!(
            "[Robot] Stopping {} Robot",
            self.robot_params.read().unwrap().name
        );

        match self.status.write() {
            Ok(mut status) => match *status {
                RobotStatus::Active => {
                    self.send_stop()?;

                    *status = RobotStatus::Stopped;

                    info!("Robot {} has been stopped", self.get_robot_name().unwrap(),);

                    // Finish strategy
                    self.robot_params.read().unwrap().strategy.finish()?;

                    Ok(())
                }
                RobotStatus::Stopped => Err("Robot is not running"),
                RobotStatus::Locked => Err("Robot is locked"),
            },
            Err(_lock_error) => Err("Lock error"),
        }
    }

    // Locks Robot and stops all its dependent threads
    pub fn lock(&self) -> Result<(), &'static str> {
        info!(
            "[Robot] Locking {} Robot",
            self.robot_params.read().unwrap().name
        );

        match self.status.write() {
            Ok(mut status) => match *status {
                RobotStatus::Active => {
                    *status = RobotStatus::Locked;
                    info!(
                        "Robot {} has been locked",
                        self.robot_params.read().unwrap().name
                    );

                    Ok(())
                }

                RobotStatus::Stopped => Err("Can't locked Robot, Robot is stopped"),

                RobotStatus::Locked => Err("Robot is already locked"),
            },
            Err(_lock_error) => Err("Lock error"),
        }
    }

    // Robot's main cycle
    fn main_cycle(&'static self) -> Result<(), &'static str> {
        // Strategy calculates orders
        match self.calc() {
            Ok((orders, sensors)) => {
                if orders.len() > 0 {
                    self.send_order(orders)?;
                }

                for sensor in sensors.into_iter() {
                    if let Err(_) = self.sensor_sender.send(SensorMsg::InfluxPoint(sensor)) {
                        continue;
                    }
                }
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn on_post_cycle(&'static self) {
        let lock = self.robot_params.write();
        match lock {
            Err(_) => {}
            Ok(mut params) => {
                let sensors = params.strategy.as_mut().sense();
                for sensor in sensors.into_iter() {
                    if let Err(_) = self.sensor_sender.send(sensor) {
                        continue;
                    }
                }
            }
        }
        // let mut tags = HashMap::new();
        // tags.insert("robot_name".into(), "sample".into());
        // let mut fields = HashMap::new();
        // fields.insert("status".into(), rand::random());
        // let point = InfluxPoint::new("RobotTestInfo".into(), tags, Utc::now(), fields);
        // self.sensor_sender.send(SensorMsg::InfluxPoint(point)).unwrap();
    }

    // Sends an order to Order Manager
    fn send_order(&self, orders: Vec<OrderContainer>) -> Result<(), &'static str> {
        // debug!(
        //     "[Robot] Robot {} is sending orders: [first order] {:?}",
        //     self.get_robot_name().unwrap(),
        //     orders.first().unwrap()
        // );
        match self.order_sender.send(OrderMsg::OrderContainers(orders)) {
            Ok(_) => {
                debug!("[Robot] Order was sent");
                Ok(())
            }
            Err(_e) => Err("Channel error"),
        }
    }

    // Calculating Robot's main algorithm
    fn calc(&self) -> Result<(Vec<OrderContainer>, Vec<InfluxPoint>), &'static str> {
        // Strategy calculation
        match self.robot_params.read().unwrap().strategy.calc() {
            Ok((actions, sensors)) => {
                let mut orders = Vec::new();
                let robot_name = self.get_robot_name().unwrap();
                for action in actions {
                    let order_params = OrderParams {
                        current_time: Utc::now().to_string(),
                    };

                    orders.push(OrderContainer {
                        robot_id: robot_name.clone(),
                        order: match action.order_type {
                            OrderType::Limit(limit) => Order::LimitOrder(LimitOrder {
                                gateway: action.exchange,
                                symbol: action.symbol,
                                amount: action.amount,
                                price: limit.price,
                                order_side: action.order_side,
                                custom_order_id: format!(
                                    "{}",
                                    OrderParams::calculate_hash(&order_params)
                                ),
                            }),
                            OrderType::Market(_market) => Order::MarketOrder(MarketOrder {
                                gateway: action.exchange,
                                symbol: action.symbol,
                                amount: action.amount,
                                order_side: action.order_side,
                            }),
                        },
                        metainfo: action.extended_strategy_params,
                        created_at: Instant::now(),
                    });
                }
                Ok((orders, sensors))
            }
            Err(error) => {
                error!("Strategy error: {}", error);

                Err("Strategy error")
            }
        }
    }

    // Receives information from Context Manager
    fn receive_info(&self) -> Result<RobotStatus, &'static str> {
        match self.context_info_receiver.try_recv() {
            Ok(context_msg) => {
                let context_info_lock = self.context_info_store.write();
                match context_info_lock {
                    Ok(mut context_info_store) => match context_msg {
                        ContextMsg::ContextInfo(context_info) => {
                            let estimated_time = context_info.created_at.elapsed();

                            GATEWAY_TO_ROBOT_TIMES.lock().unwrap().push(estimated_time);

                            let positions = context_info.positions.clone();

                            // Risk Control
                            match self.risk_control.read() {
                                Ok(risk_control) => {
                                    // Chceck Robot for risks
                                    if risk_control.check_risk(&positions) {
                                        // Lock Robot and return Status
                                        self.lock().unwrap();

                                        return Ok(RobotStatus::Locked);
                                    }

                                    // Load context info to strategy
                                    self.robot_params
                                        .read()
                                        .unwrap()
                                        .strategy
                                        .load_data(context_info.clone())?;

                                    // Store current context info
                                    *context_info_store = context_info;
                                }
                                Err(_e) => error!("Can't get access to Risk Control"),
                            }
                        }
                    },
                    Err(_e) => {
                        error!("RwLock error");
                    }
                }
                // info!("[Robot] Robot received information {:?}", self.get_robot_name());
                // debug!(
                //     "[Robot] Robot {} received information",
                //     self.get_robot_name()?
                // );
            }
            Err(_e) => {
                // Empty channel. Waiting. Do nothing
            }
        }
        Ok(RobotStatus::Active)
    }

    // Get status of Robot: Active, Stopped, Locked
    pub fn status(&self) -> Result<RobotStatus, &'static str> {
        match self.robot_params.read() {
            Ok(robot_params) => {
                info!("Getting status of {} Robot", robot_params.name);

                match self.status.read() {
                    Ok(status) => Ok((*status).clone()),
                    Err(_lock_error) => Err("Robot status lock error"),
                }
            }
            Err(err) => {
                error!("{}", err);
                Err("Can't get Robot's status")
            }
        }
    }

    // pub fn get_robot_params(&self) -> Result<RobotParams, &'static str> {
    //     match self.robot_params.read() {
    //         Ok(robot_params) => Ok(robot_params),
    //         Err(_lock_error) => Err("Robot params lock error"),
    //     }
    // }

    // pub fn get_robot_name(&self) -> Result<String, &'static str> {
    //     match self.get_robot_params() {
    //         Ok(robot_params) => Ok(robot_params.name),
    //         Err(e) => Err(e),
    //     }
    // }

    pub fn get_robot_name(&self) -> Result<String, &'static str> {
        match self.robot_params.read() {
            Ok(robot_params) => Ok(robot_params.name.clone()),
            Err(_lock_error) => Err("Robot params lock error"),
        }
    }

    // Initialize Robot with channels and loads Robot params from config file
    pub fn load(
        config_file_path: &str,
        info_receiver: Receiver<ContextMsg>,
        order_sender: Sender<OrderMsg>,
        sensor_sender: Sender<SensorMsg>,
    ) -> Result<Self, &'static str> {
        info!("Loading Robot to Platform");
        match RobotParams::from_config(config_file_path) {
            Ok(robot_params) => {
                let pnl = robot_params.pnl.clone();
                Ok(Robot {
                    robot_params: RwLock::new(robot_params),
                    status: RwLock::new(RobotStatus::Stopped),
                    risk_control: RwLock::new(RiskControl::from_robot_pnl(&pnl)),
                    context_info_receiver: info_receiver,
                    order_sender,
                    sensor_sender,
                    stop_channel: bounded(0),

                    context_info_store: RwLock::new(ContextInfo::new()),
                })
            }
            Err(_e) => Err("Robot params error"),
        }
    }

    pub fn generate(name: &str) -> Robot {
        Robot {
            robot_params: RwLock::new(RobotParams {
                name: name.to_string(),
                ..Default::default()
            }),
            ..Robot::default()
        }
    }

    // Gets information about Robot
    pub fn info(&self) -> Result<String, &'static str> {
        let robot_params_lock = self.robot_params.read().unwrap();
        Ok(format!(
            r#"Robot
name: {}
status: {:?}
"#,
            robot_params_lock.name,
            *self.status.read().unwrap(),
        ))
    }

    // Sets new parameters for Robot from config file
    pub fn set_config(&self, config_file_path: &str) -> Result<(), &'static str> {
        match self.status()? {
            RobotStatus::Active => Err("Robot is Active. Stop it before to set config"),

            // Set config only when robot is stopped
            RobotStatus::Stopped => match RobotConfig::from_file(config_file_path) {
                // If Robot config file found then validate it and set up new params
                Ok(robot_config) => match RobotParams::validate_config(&robot_config) {
                    Ok(_) => self._set_config(robot_config),
                    Err(_error) => Err("Config validation error"),
                },
                Err(e) => {
                    error!("[Robot] get config error: {}", e);
                    Err("Cannont get robot config file")
                }
            },

            RobotStatus::Locked => Err("Robot is Locked."),
        }
    }

    fn update_risk_control(&self, robot_params: &RobotParams) {
        let risk_control_lock = &mut *self.risk_control.write().unwrap();

        risk_control_lock.limits = RiskLimit {
            max_loss: robot_params.pnl.max_loss,
            stop_loss: robot_params.pnl.stop_loss,
            number_of_bad_deals: NUMBER_OF_BAD_DEALS,
            time_interval_of_bad_deals: BAD_DEAL_TIME,
            bad_deal_chain_sequence: robot_params
                .pnl
                .components
                .iter()
                .map(|c| c.bad_deal_chain_sequence)
                .collect(),
        }
    }

    // Writes new parameters for Robot
    fn _set_config(&self, robot_config: RobotConfig) -> Result<(), &'static str> {
        match self.robot_params.write() {
            Ok(mut robot_params_lock) => {
                robot_params_lock.name = robot_config.name;

                robot_params_lock.strategy_type = match robot_config.strategy.strategy_type {
                    super::config::RobotConfigStrategyType::Arbitration => {
                        RobotStrategyType::Arbitration
                    }
                    super::config::RobotConfigStrategyType::SimpleIncreaseDecrease => {
                        RobotStrategyType::SimpleIncreaseDecrease
                    }
                };

                robot_params_lock.strategy = match robot_config.strategy.strategy_type {
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

                self.update_risk_control(&*robot_params_lock);

                Ok(())
            }
            Err(_e) => Err("err"),
        }
    }
}

impl Default for Robot {
    fn default() -> Self {
        // Stubs
        let info_receiver: Receiver<ContextMsg> = unbounded().1;
        let order_sender: Sender<OrderMsg> = unbounded().0;
        let stop_channel: (Sender<()>, Receiver<()>) = bounded(0);
        let sensor_sender: Sender<SensorMsg> = unbounded().0;

        Robot {
            robot_params: RwLock::new(RobotParams::default()),
            status: RwLock::new(RobotStatus::Stopped),
            risk_control: RwLock::new(RiskControl::default()),
            context_info_receiver: info_receiver,
            order_sender,
            sensor_sender,
            stop_channel: stop_channel,

            context_info_store: RwLock::new(ContextInfo::default()),
        }
    }
}

#[derive(Hash)]
struct OrderParams {
    current_time: String,
    // robot_name: String,
    // order_side: String,
}

impl OrderParams {
    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::context_manager::ContextInfo;
    use crossbeam::channel::{bounded, unbounded};

    impl Robot {
        // Creates Robot with stub channels
        fn create_stub() -> &'static Self {
            let (_s, info_receiver): (Sender<ContextMsg>, Receiver<ContextMsg>) = unbounded();
            let (order_sender, _r): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();
            let (sensor_sender, _sensor_r): (Sender<SensorMsg>, Receiver<SensorMsg>) = unbounded();
            let stop_channel: (Sender<()>, Receiver<()>) = bounded(0);

            let _ = Box::leak(Box::new(_s));
            let _ = Box::leak(Box::new(_r));

            Box::leak(Box::new(Robot {
                robot_params: RwLock::new(RobotParams::default()),
                status: RwLock::new(RobotStatus::Stopped),
                risk_control: RwLock::new(RiskControl::default()),
                context_info_receiver: info_receiver,
                order_sender,
                sensor_sender,
                stop_channel,
                context_info_store: RwLock::new(ContextInfo::default()),
            }))
        }

        // Creates Robot with real channels
        fn create(
            info_receiver: Receiver<ContextMsg>,
            order_sender: Sender<OrderMsg>,
            sensor_sender: Sender<SensorMsg>,
        ) -> &'static Self {
            let stop_channel: (Sender<()>, Receiver<()>) = bounded(0);

            Box::leak(Box::new(Robot {
                robot_params: RwLock::new(RobotParams::default()),
                status: RwLock::new(RobotStatus::Stopped),
                risk_control: RwLock::new(RiskControl::default()),
                context_info_receiver: info_receiver,
                order_sender,
                sensor_sender,
                stop_channel,
                context_info_store: RwLock::new(ContextInfo::default()),
            }))
        }
    }

    #[test]
    fn start_robot() {
        let robot = Robot::create_stub();

        assert!(robot.status().unwrap() == RobotStatus::Stopped);
        assert!(robot.start().is_ok());
        assert!(robot.status().unwrap() == RobotStatus::Active);
    }

    #[test]
    fn stop_robot() {
        let robot = Robot::create_stub();

        assert!(robot.status().unwrap() == RobotStatus::Stopped);
        assert!(robot.start().is_ok());
        assert!(robot.status().unwrap() == RobotStatus::Active);
        assert!(robot.stop().is_ok());
        assert!(robot.status().unwrap() == RobotStatus::Stopped);
    }

    #[test]
    fn start_robot_twice() {
        let robot = Robot::create_stub();

        assert!(robot.status().unwrap() == RobotStatus::Stopped);
        assert!(robot.start().is_ok());
        assert!(robot.status().unwrap() == RobotStatus::Active);
        assert!(robot.start().is_err());
    }

    #[test]
    fn stop_robot_twice() {
        let robot = Robot::create_stub();

        assert!(robot.start().is_ok());
        assert!(robot.status().unwrap() == RobotStatus::Active);
        assert!(robot.stop().is_ok());
        assert!(robot.status().unwrap() == RobotStatus::Stopped);
        assert!(robot.stop().is_err());
        assert!(robot.status().unwrap() == RobotStatus::Stopped);
    }

    #[test]
    fn stop_robot_without_start() {
        let robot = Robot::create_stub();

        assert!(robot.status().unwrap() == RobotStatus::Stopped);
        assert!(robot.stop().is_err());
        assert!(robot.status().unwrap() == RobotStatus::Stopped);
    }

    #[test]
    fn lock_robot() {
        let robot = Robot::create_stub();

        assert!(robot.start().is_ok());
        assert!(robot.lock().is_ok());
        assert!(robot.status().unwrap() == RobotStatus::Locked);
    }

    #[test]
    fn lock_robot_twice() {
        let robot = Robot::create_stub();

        assert!(robot.status().unwrap() == RobotStatus::Stopped);
        assert!(robot.start().is_ok());
        assert!(robot.lock().is_ok());
        assert!(robot.lock().is_err());
        assert!(robot.status().unwrap() == RobotStatus::Locked);
    }

    #[test]
    fn lock_stopped_robot() {
        let robot = Robot::create_stub();

        assert!(robot.status().unwrap() == RobotStatus::Stopped);
        assert!(robot.start().is_ok());
        assert!(robot.status().unwrap() == RobotStatus::Active);
        assert!(robot.stop().is_ok());
        assert!(robot.lock().is_err());
        assert!(robot.status().unwrap() == RobotStatus::Stopped);
    }

    #[test]
    fn get_robot_name() {
        let robot = Robot::create_stub();
        assert!(robot.get_robot_name().is_ok());
    }

    #[test]
    #[ignore]
    // For local testing
    fn get_robot_name_print() {
        let robot = Robot::create_stub();

        println!("Robot name: {}", robot.get_robot_name().unwrap());
    }

    #[test]
    fn load() {
        let (_, info_receiver): (Sender<ContextMsg>, Receiver<ContextMsg>) = unbounded();
        let (order_sender, _r): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();
        let (sensor_sender, _r): (Sender<SensorMsg>, Receiver<SensorMsg>) = unbounded();

        let robot_config_file_path = "test_files/robot_config.toml";
        let robot = Robot::load(
            robot_config_file_path,
            info_receiver,
            order_sender,
            sensor_sender,
        );
        assert!(robot.is_ok());
    }

    #[test]
    #[ignore]
    // TODO make sending info
    fn main_cycle() {
        let (_s, info_receiver): (Sender<ContextMsg>, Receiver<ContextMsg>) = unbounded();
        let (order_sender, _r): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();
        let (sensor_sender, _r): (Sender<SensorMsg>, Receiver<SensorMsg>) = unbounded();

        let robot = Robot::create(info_receiver, order_sender, sensor_sender);
        assert!(robot.main_cycle().is_ok());
    }

    #[test]
    #[ignore]
    // TODO make sending info
    fn calc() {
        let robot = Robot::create_stub();

        robot.receive_info().unwrap();

        assert!(robot.calc().is_err());
    }

    #[test]
    fn calc_empty_info() {
        let robot = Robot::create_stub();

        // No data
        assert!(robot.calc().is_err());
    }

    #[test]
    #[ignore]
    // For local testing
    fn calc_print() {
        let robot = Robot::create_stub();

        println!("{:?}", robot.calc());
    }

    #[test]
    fn send_order() {
        let (_, info_receiver): (Sender<ContextMsg>, Receiver<ContextMsg>) = unbounded();

        let (order_sender, _r): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();
        let (sensor_sender, _r): (Sender<SensorMsg>, Receiver<SensorMsg>) = unbounded();

        let robot = Robot::create(info_receiver, order_sender, sensor_sender);

        assert!(robot.send_order(vec![OrderContainer::default()]).is_ok());
    }

    #[test]
    fn receive_info() {
        let robot = Robot::create_stub();
        assert!(robot.receive_info().is_ok());
    }

    #[test]
    fn status() {
        let robot = Robot::create_stub();
        assert!(robot.status().is_ok());
    }

    #[test]
    fn status_active() {
        let robot = Robot::create_stub();
        robot.start().unwrap();
        assert_eq!(RobotStatus::Active, robot.status().unwrap());
    }

    #[test]
    fn status_locked() {
        let robot = Robot::create_stub();
        robot.start().unwrap();
        robot.lock().unwrap();
        assert_eq!(RobotStatus::Locked, robot.status().unwrap());
    }

    #[test]
    #[ignore]
    // For local testing
    fn status_print() {
        let robot = Robot::create_stub();
        println!("Robot status {}", robot.status().unwrap());
    }

    #[test]
    fn info() {
        let robot = Robot::create_stub();
        assert!(robot.info().is_ok());
    }

    #[test]
    #[ignore]
    // For local testing
    fn info_print() {
        let robot = Robot::create_stub();
        println!("Robot info: {:?}", robot.info().unwrap());
    }

    #[test]
    fn set_config() {
        let robot = Robot::create_stub();
        let config_file_path = "test_files/robot_config.toml";

        assert!(robot.set_config(config_file_path).is_ok());
    }

    #[test]
    fn test_hash() {
        use chrono::Utc;

        let order_params = OrderParams {
            current_time: Utc::now().to_string(),
        };

        println!("{}", OrderParams::calculate_hash(&order_params));
    }
}
