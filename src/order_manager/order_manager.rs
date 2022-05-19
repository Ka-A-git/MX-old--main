use super::models::{
    ActiveOrderMsg, CancelOrder, Order, OrderContainer, OrderManagerState, OrderMsg,
};
use crate::context_manager::ActiveOrder;
use crate::gateway::Gateway;
use crossbeam::channel::{bounded, Receiver, Sender};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::RwLock;
use std::thread;
use std::{collections::hash_map::Entry, fs::File};
use tracing::{debug, error, info, warn};

const SENT_ORDERS_FILE_PATH: &str = "./data/active_orders.bin";

#[derive(Debug)]
pub struct OrderManager {
    // Received orders from Robots
    // <Robot ID, [orders]>
    pub(super) wanted_orders: RwLock<HashMap<String, Vec<OrderContainer>>>,

    // Orders that only sent to Gateway (not Exchange)
    // <Gateway name, [sent order containers]>
    pub(super) sent_orders: RwLock<HashMap<String, Vec<OrderContainer>>>,

    // Active open orders on Exchange
    // <Robot name, [open orders]>
    pub(super) active_orders: RwLock<HashMap<String, Vec<ActiveOrder>>>,

    // orders which will be send to gateways
    // orders_to_place: RwLock<HashMap<String, Vec<LimitOrder>>>,

    // State of Order Manager: {Started, Stopped}
    pub(super) current_state: RwLock<OrderManagerState>,

    // Channels

    // Sender orders to Gateways
    pub(super) order_msg_senders: HashMap<String, Sender<OrderMsg>>,

    // Receive orders Robots sent
    pub(super) order_msg_receiver: Receiver<OrderMsg>,
    // Receive orders that was sent to the Exchange
    pub(super) active_order_msg_receiver: Receiver<ActiveOrderMsg>,

    pub(super) ask_stop_channel: (Sender<()>, Receiver<()>),
    pub(super) send_stop_channel: (Sender<()>, Receiver<()>),
    // stop_channel: (Mutex<mpsc::Sender<()>>, Arc<Mutex<mpsc::Receiver<()>>>),
    // pub senders: HashMap<String, Sender<Vec<Order>>>,
}

impl<'a> OrderManager {
    pub fn init(
        order_msg_senders: HashMap<String, Sender<OrderMsg>>,
        order_msg_receiver: Receiver<OrderMsg>,
        active_order_msg_receiver: Receiver<ActiveOrderMsg>,
    ) -> Self {
        OrderManager {
            wanted_orders: RwLock::new(HashMap::new()),

            sent_orders: RwLock::new(HashMap::new()),
            // orders_to_place: RwLock::new(HashMap::new()),

            // orders_to_cancel: RwLock::new(HashMap::new()),

            // Channels
            order_msg_senders,
            order_msg_receiver,
            active_order_msg_receiver,

            active_orders: RwLock::new(HashMap::new()),

            // orders_to_send: RwLock::new(Vec::new()),
            ask_stop_channel: bounded(0),
            send_stop_channel: bounded(0),
            // stop_channel: (Mutex::new(sender), Arc::new(Mutex::new(receiver))),
            current_state: RwLock::new(OrderManagerState::Stopped),
        }
    }

    pub fn start(&'static self) -> Result<(), &'static str> {
        info!("Starting Order Manager");
        match self.current_state.write() {
            Ok(mut state) => match *state {
                OrderManagerState::Started => {
                    let err_msg = "Order Manager is already started";

                    Err(err_msg)
                }
                OrderManagerState::Stopped => {
                    *state = OrderManagerState::Started;

                    info!("Order Manager has started");

                    // Do tasks on start Order Manager
                    self.on_start();

                    // Runs thread for receiving orders from Robots
                    thread::spawn(move || {
                        info!("[Order Manager] Starting receive orders from Robots");

                        loop {
                            self.ask().unwrap();

                            match self.ask_stop_channel.1.try_recv() {
                                Ok(_) => {
                                    break;
                                }
                                Err(_channel_error) => {
                                    // Skip, nothing received
                                }
                            }
                        }
                    });

                    // Runs thread for sending orders to Gateways
                    thread::spawn(move || {
                        info!("[Order Manager] Starting send orders to Gateways");

                        loop {
                            self.send_to_gateways().unwrap();
                            match self.send_stop_channel.1.try_recv() {
                                Ok(_) => {
                                    info!("Order Manager stopped sending orders to gateways");
                                    break;
                                }
                                Err(_channel_error) => {
                                    // Skip, nothing received
                                }
                            }
                        }
                    });

                    // Runs thread for receiving active orders from the exchanges
                    // thread::spawn(move || {
                    //     info!("[Order Manager] Starting receive active orders from exchanges");

                    //     loop {
                    //         self.receive_active_orders().unwrap();

                    //         match self.state().unwrap() {
                    //             OrderManagerState::Started => {}
                    //             OrderManagerState::Stopped => break,
                    //         }
                    //     }
                    // });

                    Ok(())
                }
            },
            Err(_) => {
                error!("Order Manager, Lock error");

                Err("Order Manager hasn't started")
            }
        }
    }

    fn state(&self) -> Result<OrderManagerState, &'static str> {
        match self.current_state.read() {
            Ok(state) => Ok(state.clone()),
            Err(_e) => Err("Lock error"),
        }
    }

    // Receiving orders from Robots
    fn ask(&'static self) -> Result<(), &'static str> {
        match self.order_msg_receiver.try_recv() {
            Ok(order_msg) => {
                // info!("[Order Manager] Order Manager received an order message");

                match order_msg {
                    OrderMsg::OrderContainers(order_containers) => {
                        match self.wanted_orders.write() {
                            Ok(mut wanted_orders) => {
                                // let robot_id = Self::get_robot_id(&orders);

                                // All orders have the same robot_id just pick up the first one item
                                let robot_id = order_containers.first().unwrap().robot_id.clone();

                                // let orders_value = VecDeque::from(order_containers);

                                if let Some(orders) = wanted_orders.get_mut(&robot_id) {
                                    orders.extend(order_containers);
                                } else {
                                    wanted_orders.insert(robot_id, order_containers);
                                }
                            }

                            Err(e) => {
                                error!("Poison error {}", e);
                            }
                        }
                    }

                    OrderMsg::Stop => {}
                }
            }
            Err(_) => {}
        }
        Ok(())
    }

    // Send orders to Gateways
    pub fn send_to_gateways(&'static self) -> Result<(), &'static str> {
        let mut orders_by_gateway: HashMap<String, Vec<OrderContainer>> = HashMap::new();

        match self.wanted_orders.write() {
            Ok(mut wanted_orders) => {
                // If there are not wanted orders, return Ok and wait
                if wanted_orders.is_empty() {
                    return Ok(());
                }

                // let order = wanted_orders_lock.pop_front().unwrap();

                for robot_id in wanted_orders.keys() {
                    let order_containers = &wanted_orders[robot_id];

                    for order_container in order_containers {
                        let mut orders_to_send = Vec::new();

                        // Check if an order is open.
                        // Open order is an order with the same parameters that jwas already sent to Gateway
                        // We should cancel these orders
                        let orders_to_cancel = self.check_order(&order_container);

                        // Add all cancel orders for previous orders
                        orders_to_send.extend(orders_to_cancel);

                        // Add new current order
                        orders_to_send.push(order_container.clone());

                        let gateway = Gateway::extract_gateway_name(&Self::get_gateway(
                            orders_to_send.first().unwrap(),
                        ));

                        // Accumulate and group orders by gateway names
                        match orders_by_gateway.get_mut(&gateway) {
                            Some(orders) => {
                                orders.extend(orders_to_send);
                            }
                            None => {
                                orders_by_gateway.insert(gateway, orders_to_send);
                            }
                        }
                    }
                }
                wanted_orders.clear();
            }

            Err(e) => {
                error!("Poison error {}", e)
            }
        }

        // Send orders for gateways
        self.send_orders(orders_by_gateway);

        Ok(())
    }

    // Sends orders to Gateway
    fn send_orders_to_gateway(&self, gateway: &str, orders: Vec<OrderContainer>) {
        // Find gateway channel
        match self.order_msg_senders.get(gateway) {
            Some(sender) => {
                match sender.send(OrderMsg::OrderContainers(orders.clone())) {
                    Ok(_) => {
                        // info!("Orders successfully were sent to Gateway");
                        // debug!("Orders were sent: {:?}", orders_to_send);

                        // OrderUtils::inspect_orders(&orders);

                        self.save_sent_orders(gateway, orders);
                    }
                    Err(_e) => {
                        error!("Orders were not sended to Gateway, channel error");
                    }
                }
            }
            None => {
                error!("Gateway name not found");
            }
        }
    }

    // Save orders before send to gateway
    fn save_sent_orders(&self, gateway: &str, orders: Vec<OrderContainer>) {
        match self.sent_orders.write() {
            Ok(mut sent_orders) => {
                // Save limit orders only
                let limit_orders = orders
                    .iter()
                    .filter(|order_container| match order_container.order {
                        Order::LimitOrder(_) => true,
                        Order::MarketOrder(_) => false,
                        Order::CancelOrder(_) => false,
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                sent_orders.insert(gateway.to_string(), limit_orders);
            }
            Err(e) => {
                error!("Poison error {}", e)
            }
        }
    }

    // Save sent orders to persist storage before stopping platform
    fn store_sent_orders(orders: HashMap<String, Vec<OrderContainer>>) {
        match File::create(SENT_ORDERS_FILE_PATH) {
            Ok(mut active_orders) => {
                let serialized_orders = bincode::serialize(&orders).unwrap();

                active_orders.write_all(&serialized_orders).unwrap();
            }
            Err(error) => {
                error!("Can't create storage for active orders: {}", error);
            }
        }
    }

    // Load sent orders from persist storage after starting platform for cancel them
    fn load_send_orders() -> HashMap<String, Vec<OrderContainer>> {
        let mut orders = HashMap::new();

        match File::open(SENT_ORDERS_FILE_PATH) {
            Ok(mut sent_orders_file) => {
                let mut buf = Vec::new();

                sent_orders_file.read_to_end(&mut buf).unwrap();

                orders = bincode::deserialize(&buf).unwrap();
            }
            Err(error) => {
                error!("Can't open file: {}", error);
            }
        }
        orders
    }

    // Order Manager should cancel all open orders before stop platform
    // It creates cancel orders for all open orders
    fn make_cancel_orders(&self) -> HashMap<String, Vec<OrderContainer>> {
        let mut orders_to_cancel = HashMap::new();

        match self.sent_orders.write() {
            Ok(mut sent_orders) => {
                for (gateway, orders) in sent_orders.iter() {
                    orders_to_cancel.insert(
                        gateway.clone(),
                        orders
                            .iter()
                            .filter_map(|order_container| {
                                Self::convert_limit_to_cancel(order_container)
                            })
                            .collect(),
                    );
                }
                sent_orders.clear();
            }
            Err(e) => {
                error!("Order Manager poison error {}", e)
            }
        }

        orders_to_cancel
    }

    // orders: <Gateway name, vec[order containers]>
    fn send_orders(&self, grouped_orders: HashMap<String, Vec<OrderContainer>>) {
        for (gateway, orders) in grouped_orders {
            self.send_orders_to_gateway(&gateway, orders);
        }
    }

    // Do tasks on start Order Manager
    fn on_start(&self) {
        info!("[Order Manager] Do tasks on start Order Manager");

        // let open_orders = Self::load_send_orders();

        // self.sent_orders();
    }

    // Do tasks on finish Order Manager
    fn on_finish(&self) {
        info!("[Order Manager] Do tasks on finish Order Manager");
        // info!("[Order Manager] Cancel all open orders on finish Order Manager");

        // It cancels all open orders
        // Cancel orders are grouped by gateways
        let cancel_orders = self.make_cancel_orders();

        // Sends cancel orders to Gateways
        self.send_orders(cancel_orders);

        // match self.sent_orders.read() {
        //     Ok(sent_orders) => Self::store_sent_orders(orders),
        //     Err(_) => {}
        // }
    }

    // Convert Order Container from Limit Order to Cancel Order
    fn convert_limit_to_cancel(order_container: &OrderContainer) -> Option<OrderContainer> {
        if let Order::LimitOrder(limit_order) = &order_container.order {
            Some(OrderContainer {
                robot_id: order_container.robot_id.clone(),
                order: Order::CancelOrder(CancelOrder {
                    order_id: 0,
                    gateway: limit_order.gateway.clone(),
                    symbol: limit_order.symbol.clone(),
                    price: limit_order.price,
                    amount: limit_order.amount,
                    order_side: limit_order.order_side.clone(),
                    custom_order_id: limit_order.custom_order_id.clone(),
                }),
                metainfo: order_container.metainfo.clone(),
                created_at: order_container.created_at,
            })
        } else {
            None
        }
    }

    // Group orders by Gateways from Robot
    fn group_by_gateways(orders: Vec<OrderContainer>) -> HashMap<String, Vec<OrderContainer>> {
        let mut orders_by_gateways = HashMap::new();

        for order in orders {
            let gateway = Self::get_gateway(&order);

            let a = match orders_by_gateways.entry(gateway) {
                Entry::Occupied(o) => o.into_mut(),
                Entry::Vacant(v) => v.insert(vec![]),
            };
            a.push(order);
        }
        orders_by_gateways
    }

    // Receives active orders from Gateways
    fn receive_active_orders_msg(&self) -> Result<(), &'static str> {
        match self.active_order_msg_receiver.try_recv() {
            Ok(active_order_msg) => self.handle_active_order(active_order_msg),
            Err(_e) => {}
        }

        Ok(())
    }

    fn handle_active_order(&self, active_order_msg: ActiveOrderMsg) {
        match self.active_orders.write() {
            Ok(mut active_orders_lock) => match active_order_msg {
                // If it is a new active order safe it
                ActiveOrderMsg::ActiveStateOrder(active_order) => {
                    let robot_id = active_order.robot_id.clone();
                    match active_orders_lock.get_mut(&robot_id) {
                        Some(active_orders) => {
                            active_orders.push(active_order);
                        }
                        None => {
                            active_orders_lock.insert(robot_id, vec![active_order]);
                        }
                    }
                }
                // If it is a filled order remove active order
                ActiveOrderMsg::FilledOrder(_filled_order) => {
                    // let robot_id = filled_order.robot_id.clone();

                    // let robot_orders = active_orders_lock.get(&robot_id).unwrap();

                    // // Just iterate over all robot ids
                    // for order in robot_orders {
                    //     robot_orders.drain_filter(|order| order.order_id == filled_order.order_id);
                    // }
                }
            },
            Err(e) => {
                error!("Poison error {}", e);
            }
        }
    }

    pub fn stop(&self) -> Result<(), &'static str> {
        info!("Stopping Order Manager");
        match self.current_state.write() {
            Ok(mut state) => match *state {
                OrderManagerState::Started => {
                    match self.ask_stop_channel.0.send(()) {
                        Ok(_) => {
                            info!("[Order manager] Stopped receiving orders");
                        }
                        Err(_channel_error) => {
                            error!("Order Manager hasn't stopped");
                            return Err("Order Manager hasn't been stopped, channel error");
                        }
                    };

                    match self.send_stop_channel.0.send(()) {
                        Ok(_) => {
                            info!("[Order Manager] Stopped sending orders");
                        }
                        Err(_channel_error) => {
                            error!("Order Manager hasn't stopped");
                            return Err("Order Manager hasn't been stopped, channel error");
                        }
                    }

                    // Do tasks on finish Order Manager
                    self.on_finish();

                    *state = OrderManagerState::Stopped;
                    info!("Order Manager has been stopped successfully");
                    return Ok(());
                }
                OrderManagerState::Stopped => {
                    let err_msg = "Order Manager is already stopped";
                    error!("{}", err_msg);
                    Err(err_msg)
                }
            },
            Err(_) => {
                debug!("Order Manager, Lock error");
                Err("Order Manager hasn't stopped")
            }
        }
    }

    // Checks if an order is open
    // If sent orders found return new Cancel Orders
    fn check_order(&self, order_container: &OrderContainer) -> Vec<OrderContainer> {
        let robot_id = order_container.robot_id.clone();
        let mut cancel_orders = Vec::new();

        match &order_container.order {
            Order::LimitOrder(limit_order) => {
                match self.sent_orders.write() {
                    Ok(mut sent_orders) => {
                        // Gets Orders for specific gateway
                        match sent_orders.get_mut(&limit_order.gateway) {
                            Some(gateway_sent_orders) => {
                                // Gets Orders for specific robot id

                                // match gateway_sent_orders.get_mut(robot_id) {
                                //     Some(robot_sent_orders) => todo!(),
                                //     None => todo!(),
                                // }

                                //
                                let indexes =
                                    Self::find_sent_orders(&gateway_sent_orders, &order_container);

                                for index in indexes {
                                    // Remove sent order from store
                                    let sent_order = gateway_sent_orders.remove(index);

                                    cancel_orders
                                        .push(Self::convert_limit_to_cancel(&sent_order).unwrap());
                                }
                            }

                            None => {
                                // Not an error
                                warn!("Robot Id not found. Perhaps, there are no active orders for that Robot. Robot ID: {}", robot_id);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Poison error {}", e)
                    }
                }
            }

            // Don't check market orders for sent order
            Order::MarketOrder(_) => {}

            // And don't check cancel order
            Order::CancelOrder(_) => {}
        }

        cancel_orders
    }

    // Find orders that Robot(strategy) sent on previous step
    fn find_sent_orders(
        sent_containers: &Vec<OrderContainer>,
        container: &OrderContainer,
    ) -> Vec<usize> {
        let mut indexes = Vec::new();

        if let Order::LimitOrder(limit_order) = &container.order {
            for (index, sent_container) in sent_containers.iter().enumerate() {
                match &sent_container.order {
                    Order::LimitOrder(sent_limit_order) => {
                        if sent_container.robot_id == container.robot_id
                            && sent_limit_order.gateway == limit_order.gateway
                            && sent_limit_order.symbol == limit_order.symbol
                            && sent_limit_order.order_side == limit_order.order_side
                            // Check if it's the same strategy
                            && sent_container.metainfo == container.metainfo
                        {
                            indexes.push(index);
                        }
                    }

                    // Don't check market orders for sent order
                    Order::MarketOrder(_) => {}

                    // And don't check cancel order
                    Order::CancelOrder(_) => {}
                }
            }
        }
        indexes
    }

    // Gets Gatewway name filed from order
    fn get_gateway(order_container: &OrderContainer) -> String {
        match &order_container.order {
            Order::LimitOrder(limit_order) => limit_order.gateway.clone(),
            Order::MarketOrder(market_order) => market_order.gateway.clone(),
            Order::CancelOrder(cancel_order) => cancel_order.gateway.clone(),
        }
    }

    // Get Robot id from a bunch of order from Robot (OrderMsg)
    // fn get_robot_id(orders: &Vec<Order>) -> String {
    //     match orders.first() {
    //         Some(first_order) => Self::get_robot_id_from_order(first_order),
    //         None => {
    //             // unreachable!()
    //             "StubForTests".to_string()
    //         }
    //     }
    // }

    // fn get_robot_id_from_order(order: &Order) -> String {
    //     match order {
    //         Order::LimitOrder(limit_order) => limit_order.robot_id.clone(),
    //         Order::MarketOrder(market_order) => market_order.robot_id.clone(),
    //         Order::CancelOrder(cancel_order) => cancel_order.robot_id.clone(),
    //     }
    // }
}
#[cfg(test)]
mod tests {

    use super::*;
    use crate::context_manager::ActiveOrder;
    use crate::order_manager::models::{LimitOrder, OrderSide};
    use crate::order_manager::utils::OrderManagerUtils;
    use crate::robot::strategy::StrategyParams;
    use crossbeam::channel::unbounded;
    use std::time::Instant;

    #[test]
    fn ask() {
        let order_manager = OrderManagerUtils::stub();

        assert!(order_manager.ask().is_ok());
    }

    #[test]
    fn send_to_gateways() {
        let order_manager = OrderManagerUtils::stub();

        assert!(order_manager.send_to_gateways().is_ok());
    }

    #[test]
    fn start_order_manger() {
        let order_manager = OrderManagerUtils::stub();

        assert!(order_manager.state().unwrap() == OrderManagerState::Stopped);
        assert!(order_manager.start().is_ok());
        assert!(order_manager.state().unwrap() == OrderManagerState::Started);
    }

    #[test]
    fn stop_order_manger() {
        let order_manager = OrderManagerUtils::stub();

        assert!(order_manager.state().unwrap() == OrderManagerState::Stopped);
        assert!(order_manager.start().is_ok());
        assert!(order_manager.state().unwrap() == OrderManagerState::Started);
        assert!(order_manager.stop().is_ok());
        assert!(order_manager.state().unwrap() == OrderManagerState::Stopped);
    }

    #[test]
    fn start_order_manger_twice() {
        let order_manager = OrderManagerUtils::stub();

        assert!(order_manager.start().is_ok());
        assert!(order_manager.start().is_err());
    }

    #[test]
    fn stop_order_manger_twice() {
        let order_manager = OrderManagerUtils::stub();

        assert!(order_manager.start().is_ok());
        assert!(order_manager.stop().is_ok());
        assert!(order_manager.stop().is_err());
    }

    #[test]
    fn stop_order_manager_without_start() {
        let order_manager = OrderManagerUtils::stub();

        assert!(order_manager.stop().is_err());
    }

    #[test]
    fn check_order() {
        let active_orders = vec![
            // Other Robot
            ActiveOrder {
                robot_id: "Robot2".to_string(),
                order_id: 123,
                symbol: "BTCUSDT".to_string(),
                amount: 1.,
                price: 1.,
                gateway: "Binance".to_string(),
                order_side: OrderSide::Buy,
            },
            // Other Gateway
            ActiveOrder {
                robot_id: "Robot1".to_string(),
                order_id: 456,
                symbol: "BTCUSDT".to_string(),
                amount: 1.,
                price: 1.,
                gateway: "Huobi".to_string(),
                order_side: OrderSide::Buy,
            },
            // Other symbol
            ActiveOrder {
                robot_id: "Robot1".to_string(),
                order_id: 789,
                symbol: "ETHUSDT".to_string(),
                amount: 1.,
                price: 1.,
                gateway: "Binance".to_string(),
                order_side: OrderSide::Buy,
            },
            // Other Side
            ActiveOrder {
                robot_id: "Robot1".to_string(),
                order_id: 1011,
                symbol: "BTCUSDT".to_string(),
                amount: 1.,
                price: 1.,
                gateway: "Binance".to_string(),
                order_side: OrderSide::Sell,
            },
            // Found
            ActiveOrder {
                robot_id: "Robot1".to_string(),
                order_id: 1213,
                symbol: "BTCUSDT".to_string(),
                amount: 1.,
                price: 1.,
                gateway: "Binance".to_string(),
                order_side: OrderSide::Buy,
            },
        ];

        let mut robot_active_orders = HashMap::new();
        robot_active_orders.insert("Robot1".to_string(), active_orders);

        let order_manager = OrderManagerUtils::stub_active_orders(robot_active_orders);

        let order = OrderContainer {
            robot_id: "Robot1".to_string(),
            order: Order::LimitOrder(LimitOrder {
                // robot_id: "Robot1".to_string(),
                gateway: "Binance".to_string(),
                symbol: "BTCUSDT".to_string(),
                amount: 1.,
                price: 1.,
                order_side: OrderSide::Buy,
                custom_order_id: "Custom Order ID".to_string(),
            }),

            metainfo: StrategyParams::Stub,
            created_at: Instant::now(),
        };

        let close_order = OrderContainer {
            robot_id: "Robot1".to_string(),
            order: Order::CancelOrder(CancelOrder {
                // robot_id: "Robot1".to_string(),
                gateway: "Binance".to_string(),
                symbol: "BTCUSDT".to_string(),
                order_id: 1213,
                amount: 1.,
                price: 1.,
                order_side: OrderSide::Buy,
                custom_order_id: "Custom Order ID".to_string(),
            }),

            metainfo: StrategyParams::Stub,
            created_at: Instant::now(),
        };

        assert_eq!(order_manager.check_order(&order), vec![close_order]);
    }

    #[test]
    fn send_orders_to_gateway() {
        let gateway = "Gateway1";

        let order_containers = vec![OrderContainer::default()];

        let (_order_sender_from_robot, order_receiver_to_order_manager): (
            Sender<OrderMsg>,
            Receiver<OrderMsg>,
        ) = unbounded();

        let (order_sender_from_order_manager, order_receiver_to_gateway): (
            Sender<OrderMsg>,
            Receiver<OrderMsg>,
        ) = unbounded();

        let mut senders = HashMap::new();
        senders.insert(gateway.to_string(), order_sender_from_order_manager);

        let order_manager =
            OrderManagerUtils::with_channels(senders, order_receiver_to_order_manager);

        order_manager.send_orders_to_gateway(gateway, order_containers.clone());

        match order_receiver_to_gateway.recv() {
            Ok(order_msg) => assert_eq!(
                order_msg,
                OrderMsg::OrderContainers(order_containers.clone())
            ),
            Err(_) => assert!(false),
        }

        let mut sent_orders_result = HashMap::new();
        sent_orders_result.insert(gateway.to_string(), order_containers);

        assert_eq!(
            *order_manager.sent_orders.read().unwrap(),
            sent_orders_result
        );
    }
}
