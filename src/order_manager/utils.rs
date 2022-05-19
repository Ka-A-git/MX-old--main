use super::{ActiveOrderMsg, Order, OrderContainer, OrderManager, OrderMsg};
use crate::context_manager::ActiveOrder;
use crate::order_manager::models::{
    CancelOrder, LimitOrder, MarketOrder, OrderManagerState, OrderRequestType, OrderSide,
};
use crossbeam::channel::{bounded, unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::info;

pub struct OrderUtils;

impl OrderUtils {
    // Inspect sending orders when they come and go through the channels
    pub fn inspect_orders(orders: &Vec<OrderContainer>) {
        info!("-");
        info!("Length {}", orders.len());
        orders.iter().for_each(|c| match &c.order {
            Order::LimitOrder(limit) => info!(
                "Limit {} {:?} {} {}",
                limit.gateway, limit.order_side, limit.price, limit.custom_order_id
            ),
            Order::MarketOrder(_market) => todo!(),
            Order::CancelOrder(cancel) => info!(
                "Cancel {} {:?} {} {}",
                cancel.gateway, cancel.order_side, cancel.price, cancel.custom_order_id
            ),
        });
        info!("-");
    }

    pub fn get_test_order(
        _robot_id: &str,
        symbol: &str,
        order_id: Option<u64>,
        amount: f64,
        price: Option<f64>,
        gateway: &str,
        order_side: OrderSide,
        order_request_type: OrderRequestType,
        custom_order_id: &str,
    ) -> Order {
        match order_request_type {
            OrderRequestType::Limit => Order::LimitOrder(LimitOrder {
                // robot_id: robot_id.to_string(),
                gateway: gateway.to_string(),
                symbol: symbol.to_string(),
                amount,
                price: price.unwrap(),
                order_side,
                custom_order_id: custom_order_id.to_string(),
            }),
            OrderRequestType::Market => Order::MarketOrder(MarketOrder {
                // robot_id: robot_id.to_string(),
                gateway: gateway.to_string(),
                symbol: symbol.to_string(),
                amount,
                order_side,
            }),
            OrderRequestType::Cancel => Order::CancelOrder(CancelOrder {
                order_id: order_id.unwrap(),
                // robot_id: robot_id.to_string(),
                gateway: gateway.to_string(),
                symbol: symbol.to_string(),
                price: price.unwrap(),
                amount: amount,
                order_side: order_side,
                custom_order_id: custom_order_id.to_string(),
            }),
        }
    }
}

pub struct OrderManagerUtils;

impl OrderManagerUtils {
    pub fn mapper() -> HashMap<String, Sender<OrderMsg>> {
        let senders = HashMap::new();
        senders
    }

    pub fn stub() -> &'static OrderManager {
        let (stub_order_sender, stub_order_receiver, stub_active_order_receiver) =
            Self::stub_channels();

        Box::leak(Box::new(OrderManager::init(
            stub_order_sender,
            stub_order_receiver,
            stub_active_order_receiver,
        )))
    }

    // All Order Manager's params
    pub fn from_params(
        // opened_orders: HashMap<String, Vec<LimitOrder>>,
        wanted_orders: HashMap<String, Vec<OrderContainer>>,
        sent_orders: HashMap<String, Vec<OrderContainer>>,
        // orders_to_place: HashMap<String, Vec<LimitOrder>>,
        // orders_to_cancel: HashMap<String, Vec<LimitOrder>>,
        order_msg_senders: HashMap<String, Sender<OrderMsg>>,
        order_msg_receiver: Receiver<OrderMsg>,
        active_order_msg_receiver: Receiver<ActiveOrderMsg>,

        active_orders: HashMap<String, Vec<ActiveOrder>>,
        // orders_to_send: Vec<Order>,
    ) -> OrderManager {
        OrderManager {
            // opened_orders: RwLock::new(opened_orders),
            wanted_orders: RwLock::new(wanted_orders),
            sent_orders: RwLock::new(sent_orders),

            // orders_to_place: RwLock::new(orders_to_place),
            // orders_to_cancel: RwLock::new(orders_to_cancel),
            order_msg_senders,
            order_msg_receiver,
            active_order_msg_receiver,

            active_orders: RwLock::new(active_orders),
            // orders_to_send: RwLock::new(orders_to_send),
            ask_stop_channel: bounded(0),
            send_stop_channel: bounded(0),

            current_state: RwLock::new(OrderManagerState::Stopped),
        }
    }

    pub fn stub_channels() -> (
        HashMap<String, Sender<OrderMsg>>,
        Receiver<OrderMsg>,
        Receiver<ActiveOrderMsg>,
    ) {
        let (stub_order_sender, stub_order_receiver, stub_active_order_receiver): (
            HashMap<String, Sender<OrderMsg>>,
            Receiver<OrderMsg>,
            Receiver<ActiveOrderMsg>,
        ) = (
            Self::mapper(),
            crossbeam::channel::unbounded().1,
            crossbeam::channel::unbounded().1,
        );

        (
            stub_order_sender,
            stub_order_receiver,
            stub_active_order_receiver,
        )
    }

    pub fn stub_active_orders(active_orders: HashMap<String, Vec<ActiveOrder>>) -> OrderManager {
        let (stub_order_sender, stub_order_receiver, stub_active_order_receiver) =
            Self::stub_channels();

        Self::from_params(
            HashMap::new(),
            HashMap::new(),
            stub_order_sender,
            stub_order_receiver,
            stub_active_order_receiver,
            active_orders,
        )
    }

    pub fn with_channels(
        order_msg_senders: HashMap<String, Sender<OrderMsg>>,
        order_msg_receiver: Receiver<OrderMsg>,
    ) -> OrderManager {
        Self::from_params(
            HashMap::new(),
            HashMap::new(),
            order_msg_senders,
            order_msg_receiver,
            unbounded().1,
            // active_orders
            HashMap::new(),
        )
    }
}
