use super::models::{
    ActiveOrder, ContextInfo, ContextMsg, DepthInfo, FilledInfo, GatewayMsg, OrderBookInfo,
    Position,
};
use super::{DepthMsg, FilledOrder};
use crate::gateway::OrderBook;
use bincode;
use crossbeam::channel::{bounded, Receiver, Sender};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::sync::Mutex;
use std::sync::RwLock;
use std::thread::{self, Thread};
use std::time::Instant;
use tracing::{debug, error, info, warn};

lazy_static! {
    static ref LAST_RECEIVED_MESSAGE_TIME: Mutex<Instant> = Mutex::new(Instant::now());
}

const FILLED_INFO_STORAGE_FILE_PATH: &str = "./data/positions.bin";

pub struct ContextManager {
    // External calculated formulas
    pub calculated_formulas: Vec<f64>,

    // OrderBook information senders to Robots
    // <RobotID, Sender for this Robot>
    info_senders: HashMap<String, Sender<ContextMsg>>,

    // Depth information receiver from different exchanges(gateways)
    info_receiver: Receiver<GatewayMsg>,

    // Stores market data about instruments from different exchanges
    // <Gateway, <Symbol, DepthInfo>>
    depth_info: RwLock<HashMap<String, HashMap<String, DepthInfo>>>,

    // <Custom Order Id, Active Order>
    active_orders_info: RwLock<HashMap<String, ActiveOrder>>,

    // Stores Filled Info, grouped by robots
    // <Robot Id, [Filled Info]>
    filled_orders_info_store: RwLock<HashMap<String, Vec<FilledInfo>>>,

    // List of gateways on platform
    gateways: Vec<String>,

    // <RobotID, <Gateway, [Symbols]>>
    subscriptions: HashMap<String, HashMap<String, Vec<String>>>,

    current_state: RwLock<ContextManagerState>,

    publish_stop_channel: (Sender<()>, Receiver<()>),
    update_stop_channel: (Sender<()>, Receiver<()>),
}

#[derive(Clone, Debug, PartialEq)]
enum ContextManagerState {
    Started,
    Stopped,
}

impl<'a> ContextManager {
    pub fn init(
        info_senders: HashMap<String, Sender<ContextMsg>>,
        info_receiver: Receiver<GatewayMsg>,
        gateways: Vec<String>,
        subscriptions: HashMap<String, HashMap<String, Vec<String>>>,
    ) -> Self {
        ContextManager {
            calculated_formulas: vec![],

            info_senders,
            info_receiver,

            depth_info: RwLock::new(HashMap::new()),
            active_orders_info: RwLock::new(HashMap::new()),
            filled_orders_info_store: RwLock::new(Self::load_filled_info().unwrap()),

            gateways,
            subscriptions,

            current_state: RwLock::new(ContextManagerState::Stopped),

            publish_stop_channel: bounded(0),
            update_stop_channel: bounded(0),
        }
    }

    // Start Context Manager and its dependent threads
    pub fn start(&'static self) -> Result<(), &'static str> {
        info!("Starting Context Manager");

        match self.current_state.write() {
            Ok(mut state) => match *state {
                ContextManagerState::Started => {
                    let err_msg = "Context Manager is already started";
                    error!("{}", err_msg);
                    Err(err_msg)
                }
                ContextManagerState::Stopped => {
                    // Running thread for publish information to Robots
                    let publish_context_handle = thread::spawn(move || {
                        info!("[Context Manager] Starts to publish context info to Robots");

                        loop {
                            self.publish_context_info().unwrap();

                            match self.publish_stop_channel.1.try_recv() {
                                Ok(_) => {
                                    info!("Context Manager stopping publish info");
                                    break;
                                }
                                Err(_e) => {
                                    // Continue waiting
                                }
                            }

                            // thread::park();
                        }
                    });

                    // Running thread for update information, get updates from Gateways
                    thread::spawn(move || {
                        info!("[Context Manager] Starts to receive context info from Gateways");

                        loop {
                            // Starts to receive depth info or open|filled orders
                            self.receive_context_info(publish_context_handle.thread())
                                .unwrap();

                            match self.update_stop_channel.1.try_recv() {
                                Ok(_) => {
                                    info!("Context Manager stopping update info");
                                    // Upnpack publish context if it was packed
                                    publish_context_handle.thread().unpark();

                                    break;
                                }
                                Err(_e) => {
                                    // Continue waiting
                                }
                            }
                        }
                    });

                    *state = ContextManagerState::Started;

                    info!("Context Manager has started");
                    Ok(())
                }
            },
            Err(_e) => {
                debug!("Context Manager, Lock error");
                Err("Lock Error")
            }
        }
    }

    // Send Context information to Robots
    fn publish_context_info(&'static self) -> Result<(), &'static str> {
        match self.depth_info.read() {
            Ok(depth_info_lock) => {
                // let depth_info_gateways: Vec<&String> = depth_info_lock.keys().collect();

                // Check if depth_info contains info for all gateways
                // if self
                //     .gateways
                //     .iter()
                //     .all(|gateway| depth_info_gateways.contains(&gateway))
                // {

                let mut orderbooks_info = Vec::new();

                let depth_info_gateway = depth_info_lock.clone();

                for gateway in depth_info_gateway.keys() {
                    let gateway_info = depth_info_gateway.get(gateway).unwrap();
                    for symbol in gateway_info.keys() {
                        let depth_info = gateway_info.get(symbol).unwrap();

                        orderbooks_info.push(OrderBookInfo {
                            gateway_name: depth_info.gateway_name.clone(),
                            exchange_name: depth_info.exchange_name.clone(),
                            symbol: symbol.to_string(),
                            order_book: OrderBook::from_depth(
                                depth_info.depth.clone(),
                                symbol,
                                gateway,
                            ),
                        });
                    }
                }

                let filled_info = self.filled_orders_info_store.read().unwrap();

                for robot_name in self.info_senders.keys() {
                    // Positions for certain robot
                    let positions = match filled_info.get(robot_name) {
                        Some(filled_info_robot) => filled_info_robot
                            .iter()
                            .map(|filled_info| Position {
                                gateway: filled_info.gateway.clone(),
                                symbol: filled_info.symbol.clone(),
                                amount: filled_info.amount.parse().unwrap(),
                                price: filled_info.price,
                                order_side: filled_info.order_side.clone(),
                                strategy_params: filled_info.strategy_params.clone(),
                            })
                            .collect::<Vec<Position>>(),
                        None => {
                            // No filled info found
                            vec![]
                        }
                    };

                    let context_msg = ContextMsg::ContextInfo(ContextInfo {
                        orderbooks_info: orderbooks_info.clone(), // Without subscription, all orderbooks. TODO
                        positions,
                        created_at: *LAST_RECEIVED_MESSAGE_TIME.lock().unwrap(),
                    });

                    match self.info_senders[robot_name].send(context_msg.clone()) {
                        Ok(_) => {
                            // debug!(
                            // "[Context Manager] Context Manager sent OrderBook information to {} Robot",
                            // robot_name
                            // );
                        }
                        Err(e) => {
                            error!("{}", e)
                        }
                    }
                    // }

                    // depth_info_lock.clear();
                }
            }
            Err(_) => error!("error"),
        }

        // No context info, just wait and return Ok
        Ok(())
    }

    // Waits for receiving message from Gateway
    // After receiving, it unpacks publish thread
    fn receive_context_info(&self, publish_thread: &Thread) -> Result<(), &'static str> {
        match self.info_receiver.try_recv() {
            // Receiving Depth|Sent order|Filled order information from Gateways
            Ok(info_msg) => {
                publish_thread.unpark();

                self.update_context_info(info_msg)?;
            }
            Err(_e) => {}
        }
        Ok(())
    }

    fn update_context_info(&self, info_msg: GatewayMsg) -> Result<(), &'static str> {
        match info_msg {
            GatewayMsg::DepthMsg(depth_msg) => self.handle_depth(depth_msg),
            GatewayMsg::ActiveOrder(active_order) => self.handle_active_order(active_order),
            GatewayMsg::FilledOrder(filled_order) => self.handle_filled_order(filled_order),
        }
    }

    fn handle_depth(&self, depth_msg: DepthMsg) -> Result<(), &'static str> {
        debug!("[Context Manager] Got updated depth information");

        let depth_info = depth_msg.depth_info;

        *LAST_RECEIVED_MESSAGE_TIME.lock().unwrap() = depth_msg.created_at;

        match self.depth_info.write() {
            Ok(mut depth_info_lock) => {
                let mut symbol = HashMap::new();
                symbol.insert(depth_info.symbol.clone(), depth_info.clone());

                depth_info_lock.insert(depth_info.exchange_name, symbol);
            }
            Err(error) => {
                error!("Poison error: {}", error)
            }
        }

        Ok(())
    }

    fn handle_active_order(&self, active_order: ActiveOrder) -> Result<(), &'static str> {
        debug!("[Context Manager] Got Active Order");

        match self.active_orders_info.write() {
            Ok(mut active_orders_lock) => {
                let custom_order_id = active_order.custom_order_id.clone();
                active_orders_lock.insert(custom_order_id, active_order);
            }
            Err(error) => error!("Poison error: {}", error),
        }

        Ok(())
    }

    fn handle_filled_order(&self, filled_order: FilledOrder) -> Result<(), &'static str> {
        debug!("[Context Manager] Got Filled Order");

        let active_orders_lock = self.active_orders_info.read().unwrap();

        match active_orders_lock.get(&filled_order.custom_order_id) {
            Some(active_order) => {
                self.write_filled_info(active_order, &filled_order)?;

                // Saves filled info into a file
                self.save_filled_info()?;
            }
            None => error!("[Context Manager] Sent Order not found"),
        }

        Ok(())
    }

    fn write_filled_info(
        &self,
        active_order: &ActiveOrder,
        filled_order: &FilledOrder,
    ) -> Result<(), &'static str> {
        match self.filled_orders_info_store.write() {
            Ok(mut filled_orders_lock) => {
                let filled_info = FilledInfo {
                    order_id: 0,
                    custom_order_id: active_order.custom_order_id.clone(),
                    gateway: active_order.gateway.clone(),
                    robot_id: active_order.robot_id.clone(),
                    symbol: active_order.symbol.clone(),
                    amount: filled_order.amount.clone(),
                    price: active_order.price,
                    order_side: active_order.order_side.clone(),
                    strategy_params: active_order.strategy_params.clone(),
                };

                match filled_orders_lock.get_mut(&active_order.robot_id) {
                    Some(v) => {
                        v.push(filled_info);
                    }
                    None => {
                        filled_orders_lock.insert(active_order.robot_id.clone(), vec![filled_info]);
                    }
                }
            }
            Err(_) => {}
        }

        Ok(())
    }

    // Loads filled info from persistent storage on start
    fn load_filled_info() -> Result<HashMap<String, Vec<FilledInfo>>, &'static str> {
        let empty = HashMap::new();

        match File::open(FILLED_INFO_STORAGE_FILE_PATH) {
            Ok(mut filled_info_storage) => {
                let mut buf = Vec::new();

                filled_info_storage.read_to_end(&mut buf).unwrap();

                match bincode::deserialize::<HashMap<String, Vec<FilledInfo>>>(&buf) {
                    Ok(positions) => {
                        info!("Deserialize positions");

                        Ok(positions)
                    }
                    Err(error) => {
                        warn!("File is empty or damaged: {}", error);

                        info!("Load empty filled orders");

                        Ok(empty)
                    }
                }
            }
            Err(error) => {
                warn!("Can't open file: {}", error);

                info!("Load empty filled orders");

                Ok(empty)
            }
        }
    }

    // Saves filled info to persistent storage on finish or its change
    fn save_filled_info(&self) -> Result<(), &'static str> {
        match File::create(FILLED_INFO_STORAGE_FILE_PATH) {
            Ok(mut filled_info_storage) => match self.filled_orders_info_store.read() {
                Ok(filled_orders) => {
                    let serialized_orders = bincode::serialize(&*filled_orders).unwrap();

                    filled_info_storage.write_all(&serialized_orders).unwrap();
                }

                Err(error) => {
                    error!("[Context Manager] Can't read filled orders info: {}", error);
                }
            },
            Err(error) => {
                error!(
                    "[Context Manager] Can't create storage for filled orders info: {}",
                    error
                );
            }
        }

        Ok(())
    }

    pub fn stop(&self) -> Result<(), &'static str> {
        info!("Stopping Context Manager");
        match self.current_state.write() {
            Ok(mut state) => match *state {
                ContextManagerState::Started => {
                    match self.publish_stop_channel.0.send(()) {
                        Ok(_) => {}
                        Err(_e) => return Err("Channel error"),
                    }

                    match self.update_stop_channel.0.send(()) {
                        Ok(_) => {}
                        Err(_e) => return Err("Channel error"),
                    }

                    *state = ContextManagerState::Stopped;

                    Ok(())
                }
                ContextManagerState::Stopped => {
                    let err_msg = "Context Manager is already stopped";
                    error!("{}", err_msg);
                    Err(err_msg)
                }
            },
            Err(_e) => Err("Lock error"),
        }
    }

    // Get current state of Context Manager: Started, Stopped
    fn state(&self) -> Result<ContextManagerState, &'static str> {
        match self.current_state.read() {
            Ok(state) => Ok(state.clone()),
            Err(_e) => Err("Context Manager lock error"),
        }
    }

    pub fn graceful_shutdown(&self) {}
}

struct ContextManagerUtils;

impl ContextManagerUtils {
    fn stub() -> &'static ContextManager {
        let stub_info_receiver: Receiver<GatewayMsg> = crossbeam::channel::unbounded().1;

        let stub_instruments_info_senders: HashMap<String, Sender<ContextMsg>> = HashMap::new();

        Box::leak(Box::new(ContextManager::init(
            stub_instruments_info_senders,
            stub_info_receiver,
            vec![],
            HashMap::new(),
        )))
    }

    fn from_params(
        // calculated_formulas: Vec<f64>,
        instruments_info_senders: HashMap<String, Sender<ContextMsg>>,
        info_receiver: Receiver<GatewayMsg>,

        latest_info: HashMap<String, HashMap<String, DepthInfo>>,
        gateways: Vec<String>,
        subscriptions: HashMap<String, HashMap<String, Vec<String>>>,
    ) -> ContextManager {
        ContextManager {
            calculated_formulas: vec![],

            info_senders: instruments_info_senders,
            info_receiver,

            depth_info: RwLock::new(latest_info),
            active_orders_info: RwLock::new(HashMap::new()),
            filled_orders_info_store: RwLock::new(HashMap::new()),

            gateways,
            subscriptions,

            current_state: RwLock::new(ContextManagerState::Stopped),

            publish_stop_channel: bounded(0),
            update_stop_channel: bounded(0),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::context_manager::FilledOrder;

    #[test]
    fn start_context_manager() {
        let context_manager = ContextManagerUtils::stub();

        assert!(context_manager.state().unwrap() == ContextManagerState::Stopped);
        assert!(context_manager.start().is_ok());
        assert!(context_manager.state().unwrap() == ContextManagerState::Started);
    }

    #[test]
    fn stop_context_manager() {
        let context_manager = ContextManagerUtils::stub();

        assert!(context_manager.state().unwrap() == ContextManagerState::Stopped);
        assert!(context_manager.start().is_ok());
        assert!(context_manager.state().unwrap() == ContextManagerState::Started);
        assert!(context_manager.stop().is_ok());
        assert!(context_manager.state().unwrap() == ContextManagerState::Stopped);
    }

    #[test]
    fn start_context_manager_twice() {
        let context_manager = ContextManagerUtils::stub();

        assert!(context_manager.state().unwrap() == ContextManagerState::Stopped);
        assert!(context_manager.start().is_ok());
        assert!(context_manager.state().unwrap() == ContextManagerState::Started);
        assert!(context_manager.start().is_err());
    }

    #[test]
    fn stop_context_manager_twice() {
        let context_manager = ContextManagerUtils::stub();

        assert!(context_manager.state().unwrap() == ContextManagerState::Stopped);
        assert!(context_manager.start().is_ok());
        assert!(context_manager.state().unwrap() == ContextManagerState::Started);
        assert!(context_manager.stop().is_ok());
        assert!(context_manager.state().unwrap() == ContextManagerState::Stopped);
        assert!(context_manager.stop().is_err());
    }

    #[test]
    fn stop_context_manager_without_start() {
        let context_manager = ContextManagerUtils::stub();

        assert!(context_manager.state().unwrap() == ContextManagerState::Stopped);
        assert!(context_manager.stop().is_err());
        assert!(context_manager.state().unwrap() == ContextManagerState::Stopped);
    }

    #[test]
    fn publish_context_info() {
        let context_manager = ContextManagerUtils::stub();

        assert!(context_manager.publish_context_info().is_ok());
    }

    #[test]
    fn update_context_info() {
        let context_manager = ContextManagerUtils::stub();

        let gateway_msg = GatewayMsg::default();
        assert!(context_manager.update_context_info(gateway_msg).is_ok());
    }

    #[test]
    fn update_context_info_sent_order() {
        let context_manager = ContextManagerUtils::stub();

        let active_order = GatewayMsg::ActiveOrder(ActiveOrder::default());
        assert!(context_manager.update_context_info(active_order).is_ok());
    }

    #[test]
    fn update_context_info_filled_order_without_sent_order() {
        let context_manager = ContextManagerUtils::stub();

        let gateway_msg = GatewayMsg::FilledOrder(FilledOrder::default());
        assert!(context_manager.update_context_info(gateway_msg).is_ok());
    }

    #[test]
    fn update_context_info_filled_order() {
        let context_manager = ContextManagerUtils::stub();

        let active_order = GatewayMsg::ActiveOrder(ActiveOrder::default());
        let filled_order = GatewayMsg::FilledOrder(FilledOrder::default());

        context_manager.update_context_info(active_order).unwrap();
        context_manager.update_context_info(filled_order).unwrap();
    }

    #[test]
    fn get_state() {
        let context_manager = ContextManagerUtils::stub();

        assert!(context_manager.state().is_ok());
    }
}
