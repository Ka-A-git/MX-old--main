use super::exchange::{
    self,
    account::{Accounts, WebSocket},
    ExchangeAction,
};
use super::{
    ExchangeName, Fee, GatewayConfig, GatewayParams, GatewayParamsAccount, GatewayParamsActions,
    Instrument, TimeLimit,
};
use crate::{
    api::huobi::websocket_data::HuobiWS,
    config::ParseConfig,
    context_manager::{ActiveOrder, DepthInfo, DepthMsg, FilledOrder, GatewayMsg},
    gateway::exchange::PlatformTransaction,
    order_manager::{
        ActiveOrderMsg, CancelOrder, LimitOrder, Order, OrderContainer, OrderMsg, OrderSide,
    },
    platform::{self, ROBOT_TO_GATEWAY_TIMES},
    robot::RobotParamsActions,
    robot::{strategy::StrategyParams, RobotParams},
};
use binance;
use crossbeam::channel::{bounded, Receiver, Sender};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    str::FromStr,
    string::ToString,
    sync::{Arc, RwLock},
    thread,
    thread::JoinHandle,
    time::Duration,
    time::Instant,
};
use strum_macros::Display;
use tokio;
use tokio::runtime::Handle;
use tracing::{debug, error, info, warn};

const GATEWAY_RECEIVE_ORDER_TIME_INTERVAL: u32 = 1000;

// Moratorium time for sending order to the exchange in seconds
const EXCHANGE_MORATORIUM_TIME: u64 = 1;

#[derive(Clone)]
pub struct Gateway {
    gateway_params: Arc<RwLock<GatewayParams>>,
    status: Arc<RwLock<GatewayStatus>>,

    order_containers: Arc<RwLock<VecDeque<OrderContainer>>>,
    // <symbol, info>
    metadata: Arc<tokio::sync::RwLock<HashMap<String, ExchangeInstrumentInfo>>>,
    orders_receiver: Receiver<OrderMsg>,
    info_sender: Sender<GatewayMsg>,

    // Sends acitve orders to Order Manager
    active_order_sender: Sender<ActiveOrderMsg>,

    stop_channel: (Sender<()>, Receiver<()>),

    exchange: Arc<Vec<Box<dyn ExchangeAction>>>,
    account: Accounts,
    websocket: Arc<WebSocket>,
}

impl fmt::Debug for Gateway {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Gateway")
            .field("gateway_params", &self.gateway_params)
            .field("status", &self.status)
            .field("orders", &self.order_containers)
            .field("metadata", &self.metadata)
            .finish()
    }
}

#[derive(Debug, PartialEq, Clone, Display)]
pub enum GatewayStatus {
    Active,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct ExchangeInstrumentInfo {
    pub base: String,
    pub quote: String,
    pub symbol: String,
    pub precision: u8,
}

// General Depth struct for all exchanges
#[derive(Clone, Debug)]
pub struct Depth {
    pub exchange: String,
    pub bids: Vec<Ticker>,
    pub asks: Vec<Ticker>,
}

impl Default for Depth {
    fn default() -> Self {
        Depth {
            exchange: "StubExchange".to_string(),
            bids: vec![Ticker::default()],
            asks: vec![Ticker::default()],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ticker {
    pub price: f64,
    pub qty: f64,
}

impl Default for Ticker {
    fn default() -> Self {
        Ticker {
            price: 1.1,
            qty: 10.01,
        }
    }
}

impl Gateway {
    // Initialize gateway with channels and loads its parameters from config file
    pub fn load(
        config_file_path: &str,
        info_sender: Sender<GatewayMsg>,
        orders_receiver: Receiver<OrderMsg>,
        active_order_sender: Sender<ActiveOrderMsg>,
    ) -> Result<Self, &'static str> {
        info!("Loading gateway");
        let gateway_params = GatewayParams::from_config(config_file_path)?;

        Ok(Gateway {
            gateway_params: Arc::new(RwLock::new(
                gateway_params.clone(), // GatewayParams::from_config(config_file_path).unwrap(),
            )),
            status: Arc::new(RwLock::new(GatewayStatus::Stopped)),
            order_containers: Arc::new(RwLock::new(VecDeque::new())),
            info_sender,
            orders_receiver,
            active_order_sender,
            metadata: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            stop_channel: bounded(0),
            exchange: Arc::new(vec![]),
            account: Accounts::get(&gateway_params),
            websocket: Arc::new(WebSocket::get(&gateway_params)),
        })
    }

    // Starts gateway and all its dependent threads
    pub fn start(&'static self) -> Result<JoinHandle<()>, &'static str> {
        let gateway_status_lock = self.status.write();
        let gateway_params_lock = self.gateway_params.read().unwrap();

        let gateway_name = gateway_params_lock.name.clone();
        let exchange = gateway_params_lock.exchange.clone();

        let symbols = gateway_params_lock
            .instruments
            .iter()
            .map(|instrument| instrument.name.clone())
            .collect::<Vec<String>>();

        let symbol = symbols.first().unwrap().clone();

        match gateway_status_lock {
            Ok(mut status) => match *status {
                GatewayStatus::Active => Err(Box::leak(Box::new(format!(
                    "Gateway {} is already running",
                    gateway_name
                )))),

                GatewayStatus::Stopped => {
                    info!("Gateway {} is starting", gateway_name.clone());

                    // Creates static variable before move in thread for logging
                    let gateway_name_log: &'static str = Box::leak(Box::new(gateway_name));

                    let current_exchange: &'static ExchangeName = Box::leak(Box::new(exchange));

                    let current_symbol: &'static str = Box::leak(Box::new(symbol));

                    // Working with actix
                    // let rt = tokio::runtime::Builder::new_current_thread()
                    //     .build()
                    //     .unwrap();

                    // Working for demo
                    let rt = Handle::current();

                    // Fetches metadata at once from exchange
                    rt.spawn(async move { self.fetch_metadata(current_exchange).await });

                    rt.spawn(async move {
                        let log_items = format!(
                            "Received metadata: {} items from {} exchange",
                            self.metadata.read().await.len(),
                            gateway_name_log
                        );
                        debug!("{}", log_items);
                    });

                    // Runs thread for receiving orders from Order Manager
                    let handle = thread::spawn(move || {
                        //  let gateway_params_lock = self.gateway_params.read().unwrap();
                        info!("Gateway {} starts receiving orders.", gateway_name_log);

                        loop {
                            self.receive_order().unwrap();
                            match self.stop_channel.1.try_recv() {
                                Ok(_) => break,
                                _ => {}
                            }
                        }
                    });

                    // Runs thread for sending orders to exchange
                    let _ = thread::spawn(move || {
                        info!(
                            "Gateway {} starts sending orders to exchanges",
                            gateway_name_log
                        );

                        loop {
                            self.send_order(current_exchange).unwrap();

                            match self.stop_channel.1.try_recv() {
                                Ok(_) => break,
                                _ => {}
                            }
                        }
                    });

                    // Runs thread for sending info to Context Manager
                    let _ = thread::spawn(move || {
                        info!(
                            "Gateway {} starts sending info to Context Manager",
                            gateway_name_log
                        );

                        self.send_info(current_symbol, current_exchange).unwrap();
                    });

                    // Runs thread for receiving filled orders from exchange
                    // and send them to Context Manager
                    let _ = thread::spawn(move || {
                        self.receive_filled_orders().unwrap();
                    });

                    *status = GatewayStatus::Active;

                    Ok(handle)
                }
            },
            Err(lock_error) => Err(Box::leak(Box::new(format!("Lock error {}", lock_error)))),
        }
    }

    // Receives filled orders from exchanges
    fn receive_filled_orders(&self) -> Result<(), &'static str> {
        // todo reads gateway params once and then pass as function argument
        let gateway_params_lock = self.gateway_params.read().unwrap();
        let config_account = gateway_params_lock.accounts.first().unwrap();

        match gateway_params_lock.exchange {
            ExchangeName::Binance => {
                // Binance api is listening all instruments
                self.binance_ws(config_account);
            }
            ExchangeName::Huobi => {
                let symbols = gateway_params_lock
                    .instruments
                    .iter()
                    .map(|instrument| instrument.name.as_str())
                    .collect::<Vec<&str>>();

                self.huobi_ws(config_account, symbols);
            }
            ExchangeName::BitMEX => {
                // TODO
            }
            ExchangeName::StubExchange => {
                info!("[Gateway] Stub Exchange: receive filled orders");
            }
        }

        Ok(())
    }

    fn binance_ws(&self, config_account: &GatewayParamsAccount) {
        use binance::websockets::WebsocketEvent;
        use exchange::binance::Binance;

        // Binance handler
        let handler = |event: WebsocketEvent| {
            match event {
                WebsocketEvent::OrderTrade(trade) => {
                    if trade.execution_type == "TRADE" {
                        info!(
                            "[Gateway] Binance order was filled: {} {}",
                            trade.symbol, trade.qty
                        );

                        match self.info_sender.send(GatewayMsg::FilledOrder(FilledOrder {
                            order_id: trade.order_id,
                            custom_order_id: trade.new_client_order_id,
                            symbol: trade.symbol.clone(),
                            amount: trade.qty.clone(),
                        })) {
                            Ok(_) => {
                                info!("[Gateway] Binance Filled Order info was sent to Context Manager");

                                // debug!(
                                //     "[Gateway] Gateway {} sent Filled Order (Binance Exchange) info {} symbol",
                                //     self.get_gateway_name().unwrap(),
                                //     trade.symbol,
                                // );
                            }
                            Err(_e) => {}
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        };

        // Run binance websockets listening
        Binance::user_stream_ws(config_account, handler);
    }

    fn huobi_ws(&self, config_account: &GatewayParamsAccount, symbols: Vec<&str>) {
        use crate::api::huobi::models::EventType;
        use crate::api::huobi::websocket_account::WebsocketEvent;
        use exchange::huobi::Huobi;

        let handler = |event: WebsocketEvent| {
            match event {
                WebsocketEvent::OrderUpdate(order_subscription) => {
                    match order_subscription.data {
                        EventType::Trade(trade) => {
                            info!(
                                "[Gateway] Huobi order was filled: {} {}",
                                trade.symbol, trade.order_size
                            );

                            match self.info_sender.send(GatewayMsg::FilledOrder(FilledOrder {
                                order_id: trade.order_id,
                                custom_order_id: trade.client_order_id,
                                symbol: trade.symbol.clone(),
                                amount: trade.order_size.clone(),
                            })) {
                                Ok(_) => {
                                    info!(
                                        "[Gateway] Huobi Filled Order info was sent to Context Manager"
                                    );
                                    // debug!(
                                    //     "[Gateway] Gateway {} sent Filled Order (Huobi Exchange) info {} symbol",
                                    //     self.get_gateway_name().unwrap(),
                                    //     trade.symbol,
                                    // );
                                }
                                Err(_e) => {}
                            }
                        }
                        EventType::Creation(_order) => {}

                        EventType::Cancellation(_order) => {}
                    };
                }
            }

            Ok(())
        };

        Huobi::user_stream_ws(config_account, handler, symbols);
    }

    fn fetch_depth(
        &'static self,
        symbol: String,
        exchange: &ExchangeName,
    ) -> Result<(), &'static str> {
        let symbols: Vec<_> = vec![symbol.as_str()];
        let gateway = self.get_gateway_name().unwrap();

        match exchange {
            ExchangeName::Binance => {
                use binance::websockets::WebsocketEvent;
                use exchange::binance::Binance;

                let handler = |event: WebsocketEvent| {
                    if let WebsocketEvent::OrderBook(order_book) = event {
                        // Orderbook has delivered, start estimate time from now
                        let created_at = Instant::now();

                        let depth = Binance::get_depth(&order_book);

                        self.info_sender(symbol.clone(), &gateway, "Binance", depth, created_at);
                    }
                    Ok(())
                };

                Binance::depth_ws(handler, symbols);
            }

            ExchangeName::Huobi => {
                // info!("Depth was fetched from Huobi successfully");

                let mut huobi_ws = HuobiWS::connect(&symbol);

                thread::spawn(move || loop {
                    let huobi_depth = huobi_ws.get_depth();

                    let depth = exchange::huobi::Huobi::get_depth(&huobi_depth);

                    let created_at = Instant::now();

                    self.info_sender(symbol.clone(), &gateway, "Huobi", depth, created_at);
                });
            }

            ExchangeName::BitMEX => {
                info!("Depth was fetched from BitMEX successfully");
            }

            ExchangeName::StubExchange => {
                info!("Depth was fetched from StubExchange successfully");
                // Do nothing
                let _depth = Depth {
                    exchange: "StubExchange".to_string(),
                    bids: vec![],
                    asks: vec![],
                };
            }
        }
        Ok(())
    }

    fn info_sender(
        &self,
        symbol: String,
        gateway: &str,
        exchange: &str,
        depth: Depth,
        created_at: Instant,
    ) {
        // info!("Info {} {:#?}", gateway, depth);
        match self.info_sender.send(GatewayMsg::DepthMsg(DepthMsg {
            depth_info: DepthInfo {
                gateway_name: gateway.to_string(),
                exchange_name: exchange.to_string(),
                symbol: symbol.clone(),
                depth,
            },
            created_at,
        })) {
            Ok(_) => {
                // info!("[Gateway] Depth info was sent to Context Manager");
                // debug!(
                // "[Gateway] Gateway {} sent Depth info {} symbol",
                // self.get_gateway_name().unwrap(),
                // symbol,
                // );
            }
            Err(e) => {
                error!("[Gateway] Error to send info: {:?}", e);
            }
        }
    }

    /// Receives order from Order Manager
    fn receive_order(&self) -> Result<(), &'static str> {
        match self.orders_receiver.try_recv() {
            Ok(order_msg) => {
                match order_msg {
                    OrderMsg::OrderContainers(received_order_containers) => {
                        // info!("Gateway received order");

                        debug!(
                            "Gateway {} received {} order containers",
                            self.get_gateway_name()?,
                            &received_order_containers.len(),
                        );

                        // crate::order_manager::utils::OrderUtils::inspect_orders(
                        //     &received_order_containers,
                        // );

                        received_order_containers
                            .iter()
                            .for_each(|c| match &c.order {
                                Order::LimitOrder(_limit) => {
                                    let time = c.created_at.elapsed();

                                    ROBOT_TO_GATEWAY_TIMES.lock().unwrap().push(time);
                                }
                                Order::MarketOrder(_market) => {}
                                Order::CancelOrder(_cancel) => {}
                            });

                        match self.order_containers.write() {
                            Ok(mut order_containers) => {
                                order_containers.extend(received_order_containers);

                                // info!(
                                //     "[Receive] {} order_containers len {}",
                                //     self.get_gateway_name().unwrap(),
                                //     order_containers.len()
                                // );
                            }
                            Err(e) => error!("Poison error {}", e),
                        }
                    }
                    OrderMsg::Stop => {}
                }
            }
            Err(_error) => {
                // Do nothing, channel is empty, no messages
            }
        }
        Ok(())
    }

    /// Send order to exchanges
    fn send_order(&'static self, exchange: &'static ExchangeName) -> Result<(), &'static str> {
        // let gateway_params_lock = self.gateway_params.read().unwrap();

        match self.order_containers.write() {
            Ok(mut order_containers) => {
                // crate::order_manager::utils::OrderUtils::inspect_orders(&Vec::from(
                //     order_containers.clone(),
                // ));

                while !order_containers.is_empty() {
                    // info!(
                    //     "[Send] {} order_containers len {}",
                    //     self.get_gateway_name().unwrap(),
                    //     order_containers.len()
                    // );

                    let order_container = order_containers.pop_front().unwrap();

                    // let _order_metainfo = order_container.metainfo;

                    // let _account = gateway_params_lock.accounts.first().unwrap();

                    let send_res = self.order_sender(order_container, exchange);
                    if let Err(e) = send_res {
                        error!("Error on order send {:?}", e);
                    }
                }
                // No orders
                Ok(())
            }
            Err(e) => {
                error!("Gateway poison error: {}", e);
                Ok(())
            }
        }
    }

    fn order_sender(
        &'static self,
        order_container: OrderContainer,
        exchange: &'static ExchangeName,
    ) -> Result<(), &'static str> {
        let robot_id = order_container.robot_id;
        let strategy_params = order_container.metainfo;
        let order = order_container.order;

        return match order {
            Order::LimitOrder(limit_order) => {
                // match self.check_balance(limit_order.clone(), robot_id.clone(), order_metainfo)
                // {
                // Ok(_) => {

                tokio::spawn(async move {
                    let precision = self
                        .metadata
                        .read()
                        .await
                        .get(&limit_order.symbol)
                        .unwrap()
                        .precision as usize;

                    // Convert price to two decimal points
                    let converted_price = format!("{:.1$}", limit_order.price, precision)
                        .parse::<f64>()
                        .unwrap();

                    let prepared_order = LimitOrder {
                        price: converted_price,
                        ..limit_order.clone()
                    };

                    let order_responce = match prepared_order.order_side {
                        OrderSide::Buy => self.limit_buy(&prepared_order, exchange),

                        OrderSide::Sell => self.limit_sell(&prepared_order, exchange),
                    };

                    match order_responce {
                        Ok(_platform_transaction) => {
                            let active_order = ActiveOrder {
                                robot_id: robot_id.to_string(),
                                custom_order_id: limit_order.custom_order_id.clone(),
                                symbol: limit_order.symbol.to_string(),
                                amount: limit_order.amount,
                                price: limit_order.price,
                                gateway: limit_order.gateway,
                                order_side: limit_order.order_side,
                                strategy_params: strategy_params.clone(),
                            };

                            // Send active order to Order Manager
                            self.save_active_order(active_order);
                        }
                        Err(e) => error!(e),
                    }
                });

                Ok(())

                // }

                //     Err(error) => {
                //         info!("Wrong balance: {}", error);
                //         info!("Order will send again after moratorium time");

                //         // Do not throw error, order will send again after moratorium time
                //         Ok(())
                //     }
                // }
            }

            Order::MarketOrder(market_order) => {
                match market_order.order_side {
                    OrderSide::Buy => {
                        self.market_buy(&market_order.symbol, market_order.amount, exchange)?
                    }
                    OrderSide::Sell => {
                        self.market_sell(&market_order.symbol, market_order.amount, exchange)?
                    }
                }

                Ok(())
            }

            Order::CancelOrder(cancel_order) => {
                self.cancel_order(&cancel_order, exchange)?;

                Ok(())
            }
        };
    }

    // TODO split thread from function, and make accept one parameter(OrderContainer or Order)
    fn check_balance(
        &'static self,
        limit_order: LimitOrder,
        robot_id: String,
        order_metainfo: StrategyParams,
    ) -> Result<(), &'static str> {
        let gateway_params_lock = self.gateway_params.read().unwrap();

        let account_params = gateway_params_lock.accounts.first().unwrap();

        let balances = self.fetch_balances()?;

        // Working with first only instrument
        let instrument = gateway_params_lock.instruments.first().unwrap();

        let total_price = limit_order.amount * limit_order.price;

        match balances.get(&account_params.name) {
            Some(account) => {
                let symbol = match limit_order.order_side {
                    OrderSide::Buy => instrument.quote.clone(),
                    OrderSide::Sell => instrument.base.clone(),
                };

                let balance = *account.get(symbol.as_str()).unwrap();

                // If total price of order more than account balance then set moratorium time for that
                if total_price > balance {
                    thread::spawn(move || {
                        thread::sleep(Duration::from_secs(EXCHANGE_MORATORIUM_TIME));
                        let mut order_containers_lock = self.order_containers.write().unwrap();
                        order_containers_lock.push_back(OrderContainer {
                            robot_id: robot_id.to_string(),
                            order: Order::LimitOrder(limit_order),
                            metainfo: order_metainfo,
                            created_at: Instant::now(),
                        });
                    });
                    error!("Not enough balance on account {}", account_params.name);
                    return Err("Not enough balance");
                }
            }

            None => {
                return Err("Account not found");
            }
        }

        Ok(())
    }

    // For next version, without base and quote fields in config
    // balance <symbol, available_balance>
    async fn _check_balance(&self, _balances: HashMap<String, f64>, limit_order: &LimitOrder) {
        // fn _check_balance(&self, _balances: HashMap<String, f64>, limit_order: &LimitOrder) {
        let exchange_instruments_info = self.metadata.read().await;
        for (_symbol, instrument_info) in exchange_instruments_info.iter() {
            if instrument_info.base == limit_order.symbol {
                match limit_order.order_side {
                    OrderSide::Buy => {
                        // limit_order.symbol
                    }
                    OrderSide::Sell => {}
                };
            }
        }
    }

    /// Fetch metadata once when gateway starts
    async fn fetch_metadata(
        &'static self,
        exchange: &'static ExchangeName,
    ) -> Result<(), &'static str> {
        info!("Fetching metadata from exchange");

        let mut metadata_lock = self.metadata.write().await;

        match exchange {
            // Fetch metadata from Binance exchange
            ExchangeName::Binance => {
                tokio::task::block_in_place(|| {
                    let binance_metadata = exchange::binance::Binance::metadata().unwrap();

                    for symbol_info in binance_metadata.symbols {
                        metadata_lock.insert(
                            symbol_info.symbol.clone(),
                            ExchangeInstrumentInfo {
                                base: symbol_info.base_asset,
                                quote: symbol_info.quote_asset,
                                symbol: symbol_info.symbol,
                                precision: exchange::binance::Binance::price_precision(
                                    symbol_info.filters,
                                ),
                            },
                        );
                    }

                    debug!("Binance metadata {:?}", self.metadata);
                });
            }
            // Fetch metadata from Huobi exchange
            ExchangeName::Huobi => {
                let huobi_metadata = exchange::huobi::Huobi::metadata().await.unwrap();

                for symbol_info in huobi_metadata.data {
                    metadata_lock.insert(
                        symbol_info.symbol.clone(),
                        ExchangeInstrumentInfo {
                            base: symbol_info.base,
                            quote: symbol_info.quote,
                            symbol: symbol_info.symbol,
                            precision: symbol_info.price_precision,
                        },
                    );
                }

                debug!("Huobi metadata {:?}", self.metadata);
            }

            // Fetch metadata from BitMEX exchange
            ExchangeName::BitMEX => {}

            // Other exchanges here

            // Stub exchange for the local testing
            ExchangeName::StubExchange => {
                // Do nothing
            }
        }
        Ok(())
    }

    // Sends info (depth) to Context Manager
    fn send_info(
        &'static self,
        symbol: &str,
        current_exchange: &ExchangeName,
    ) -> Result<(), &'static str> {
        // Fetchs depth from exchange and send to Context Manager
        self.fetch_depth(symbol.to_string(), current_exchange)?;

        Ok(())
    }

    // It returns <account, <instrument, balance>>
    fn fetch_balances(&self) -> Result<HashMap<String, HashMap<String, f64>>, &'static str> {
        let gateway_params_lock = self.gateway_params.read().unwrap();
        // info!("[Gateway] Fetching accounts balance");

        let mut balances = HashMap::new();

        let instruments = &gateway_params_lock.instruments;
        let accounts = &gateway_params_lock.accounts;

        for account in accounts {
            let mut instrument_balances = HashMap::new();
            for instrument in instruments {
                match gateway_params_lock.exchange {
                    // Fetch balacnes from Binance exchange
                    ExchangeName::Binance => {
                        let binance_account: &binance::account::Account =
                            self.account.binance.as_ref().unwrap();

                        let balance_base = binance_account.get_balance(&instrument.base);
                        let balance_quote = binance_account.get_balance(&instrument.quote);

                        info!("[Gateway] Got balance for Binance account");

                        instrument_balances.insert(
                            instrument.base.clone(),
                            balance_base.unwrap().free.parse().unwrap(),
                        );

                        instrument_balances.insert(
                            instrument.quote.clone(),
                            balance_quote.unwrap().free.parse().unwrap(),
                        );

                        balances.insert(account.name.clone(), instrument_balances.clone());
                    }

                    // Fetch balacnes from Huobi exchange
                    ExchangeName::Huobi => {
                        let huobi_account = self.account.huobi.as_ref().unwrap();

                        let balance_base =
                            huobi_account.get_balance(&instrument.base).unwrap().balance;

                        let balance_quote = huobi_account
                            .get_balance(&instrument.quote)
                            .unwrap()
                            .balance;

                        info!("[Gateway] Got balance for Huobi account");

                        instrument_balances.insert(instrument.base.clone(), balance_base);
                        instrument_balances.insert(instrument.quote.clone(), balance_quote);

                        balances.insert(account.name.clone(), instrument_balances.clone());
                    }

                    ExchangeName::BitMEX => {}

                    // Stub Exchange for the local testing
                    ExchangeName::StubExchange => {
                        instrument_balances.insert("BTC".to_string(), 100f64);

                        instrument_balances.insert("USDT".to_string(), 150f64);

                        balances.insert(
                            GatewayParamsAccount::default().name,
                            instrument_balances.clone(),
                        );
                    }
                }
            }
        }

        info!("[Gateway] Balances {:?} ", balances);

        Ok(balances)
    }

    // Send a limit buy request to buy an instrument on exchange
    fn limit_buy(
        &self,
        limit_order: &LimitOrder,
        exchange: &ExchangeName,
    ) -> Result<PlatformTransaction, &'static str> {
        info!(
            "[Gateway] Limit Buy: {} {} by {:?} on {}",
            limit_order.symbol, limit_order.amount, limit_order.price, exchange
        );

        let active_order = match exchange {
            // Send an order to Binance exchange
            ExchangeName::Binance => {
                let binance_account: &binance::account::Account =
                    self.account.binance.as_ref().unwrap();

                // match binance_account.limit_buy(symbol, qty, price) {
                match binance_account.custom_order(
                    limit_order.symbol.clone(),
                    limit_order.amount,
                    limit_order.price,
                    None,
                    binance::account::OrderSide::Buy,
                    binance::account::OrderType::Limit,
                    binance::account::TimeInForce::GTC,
                    Some(limit_order.custom_order_id.clone()),
                ) {
                    Ok(transaction) => {
                        debug!("[Binance] Ok. Limit Buy order was placed");

                        Ok(PlatformTransaction {
                            symbol: transaction.symbol,
                            order_id: transaction.order_id,
                        })
                    }
                    Err(e) => Err(Box::leak(Box::new(e)).description()),
                }
            }

            // Send an order to Huobi exchange
            ExchangeName::Huobi => {
                let huobi_account = self.account.huobi.as_ref().unwrap();

                match huobi_account.limit_buy(
                    &limit_order.symbol,
                    limit_order.amount,
                    limit_order.price,
                    Some(limit_order.custom_order_id.clone()),
                ) {
                    Ok(transaction) => {
                        debug!("[Huobi] Ok. Limit Buy order was placed");

                        Ok(PlatformTransaction {
                            symbol: transaction.symbol,
                            order_id: transaction.order_id,
                        })
                    }
                    Err(e) => Err(e),
                }
            }

            // Send an order to BitMEX exchange
            ExchangeName::BitMEX => {
                let _bitmex_account = self.account.bitmex.unwrap();

                Ok(PlatformTransaction::default())
            }

            ExchangeName::StubExchange => Ok(PlatformTransaction::default()),
        };

        active_order
    }

    // Send a market buy request to buy an instrument on exchange
    fn market_buy(
        &self,
        symbol: &str,
        qty: f64,
        exchange: &ExchangeName,
    ) -> Result<(), &'static str> {
        info!("[Gateway] Market Buy: {} {} on {}", symbol, qty, exchange);

        return match exchange {
            // Send a market order to Binance exchange
            ExchangeName::Binance => {
                let binance_account: &binance::account::Account =
                    self.account.binance.as_ref().unwrap();

                match binance_account.market_buy(symbol, qty) {
                    Ok(_transaction) => {
                        // info!("[Binance] Ok. Market Buy order");

                        Ok(())
                    }
                    Err(e) => {
                        dbg!(&e);
                        Err(Box::leak(Box::new(e)).description())
                    }
                }
            }

            // Send a market order to Huobi exchange
            ExchangeName::Huobi => {
                let huobi_account = self.account.huobi.as_ref().unwrap();

                match huobi_account.market_buy(symbol, qty) {
                    Ok(_transaction) => {
                        // info!("[Huobi] Ok. Market Buy order");

                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }

            // Send a market order to BitMEX exchange
            ExchangeName::BitMEX => {
                let _bitmex_account = self.account.bitmex.unwrap();

                Ok(())
            }

            ExchangeName::StubExchange => Ok(()),
        };
    }

    // Send a limit sell request to sell an instrument on exchange
    fn limit_sell(
        &self,
        limit_order: &LimitOrder,
        exchange: &ExchangeName,
    ) -> Result<PlatformTransaction, &'static str> {
        // info!(
        //     "[Gateway] Limit Sell: {} {} by {:?} on {} Robot {}",
        //     symbol, qty, price, exchange, robot_id,
        // );

        info!(
            "[Gateway] Limit Sell: {} {} by {:?} on {}",
            limit_order.symbol, limit_order.amount, limit_order.price, exchange
        );

        return match exchange {
            // Send an order to Binance exchange
            ExchangeName::Binance => {
                let binance_account: &binance::account::Account =
                    self.account.binance.as_ref().unwrap();

                // match binance_account.limit_sell(symbol, qty, price) {

                match binance_account.custom_order(
                    limit_order.symbol.clone(),
                    limit_order.amount,
                    limit_order.price,
                    None,
                    binance::account::OrderSide::Sell,
                    binance::account::OrderType::Limit,
                    binance::account::TimeInForce::GTC,
                    Some(limit_order.custom_order_id.clone()),
                ) {
                    Ok(transaction) => {
                        // info!("[Binance] Ok. Limit Sell order was placed");

                        Ok(PlatformTransaction {
                            symbol: transaction.symbol,
                            order_id: transaction.order_id,
                        })
                    }
                    Err(error) => {
                        error!("Binance Limit Sell error: {}", error);
                        Err(Box::leak(Box::new(error)).description())
                    }
                }
            }

            // Send an order to Huobi exchange
            ExchangeName::Huobi => {
                let huobi_account = self.account.huobi.as_ref().unwrap();

                match huobi_account.limit_sell(
                    &limit_order.symbol,
                    limit_order.amount,
                    limit_order.price,
                    Some(limit_order.custom_order_id.clone()),
                ) {
                    Ok(transaction) => {
                        // info!("[Huobi] Ok. Limit Sell order was placed");

                        Ok(PlatformTransaction {
                            symbol: transaction.symbol,
                            order_id: transaction.order_id,
                        })
                    }
                    Err(error) => {
                        error!("Huobi Limit Sell error: {}", error);

                        Err(error)
                    }
                }
            }

            // Send an order to BitMEX exchange
            ExchangeName::BitMEX => {
                let _bitmex_account = self.account.bitmex.unwrap();

                Ok(PlatformTransaction::default())
            }

            ExchangeName::StubExchange => Ok(PlatformTransaction::default()),
        };
    }

    // Send a market sell request to sell an instrument on exchange
    fn market_sell(
        &self,
        symbol: &str,
        qty: f64,
        exchange: &ExchangeName,
    ) -> Result<(), &'static str> {
        info!("[Gateway] Market Sell: {} {} on {}", symbol, qty, exchange);

        return match exchange {
            // Send a market order to Binance exchange
            ExchangeName::Binance => {
                let binance_account: &binance::account::Account =
                    self.account.binance.as_ref().unwrap();

                match binance_account.market_sell(symbol, qty) {
                    Ok(_transaction) => Ok(()),
                    Err(e) => Err(Box::leak(Box::new(e)).description()),
                }
            }

            // Send a market order to Huobi exchange
            ExchangeName::Huobi => {
                let huobi_account = self.account.huobi.as_ref().unwrap();

                match huobi_account.market_sell(symbol, qty) {
                    Ok(_transaction) => Ok(()),
                    Err(e) => Err(e),
                }
            }

            // Send a market order to BitMEX exchange
            ExchangeName::BitMEX => {
                let _bitmex_account = self.account.bitmex.unwrap();

                Ok(())
            }

            ExchangeName::StubExchange => Ok(()),
        };
    }

    // Sends active order metainfo to Order Manager
    fn save_active_order(&self, active_order: ActiveOrder) {
        match self
            .active_order_sender
            .send(ActiveOrderMsg::ActiveStateOrder(active_order))
        {
            Ok(_) => debug!("[Gateway] Active order was sent to Order Manager"),
            Err(e) => {
                error!("channel error: {}", e);
            }
        }
    }

    fn cancel_order(
        &self,
        cancel_order: &CancelOrder,
        exchange: &ExchangeName,
    ) -> Result<(), &'static str> {
        // info!(
        //     "[Gateway] Canceling order: Cancel Order {:#?} exchange {}",
        //     cancel_order, exchange
        // );

        let symbol = cancel_order.symbol.clone();
        let custom_order_id = cancel_order.custom_order_id.clone();
        let price = format!("{:.2}", cancel_order.price).parse::<f64>().unwrap();
        let amount = cancel_order.amount;
        let order_side = cancel_order.order_side.clone();

        info!(
            "[Gateway] Cancel Order: {:?} {} {} by {} on {:?}",
            order_side, symbol, amount, price, exchange
        );

        let debug_log = format!(
            "[Gateway] Canceled order: {:?} {} {} by {} on {:?}",
            order_side, symbol, amount, price, exchange
        );

        match exchange {
            ExchangeName::Binance => {
                let binance_account: &binance::account::Account =
                    self.account.binance.as_ref().unwrap();

                match binance_account.cancel_order_with_client_id(&symbol, custom_order_id) {
                    Ok(_order_canceled) => {
                        debug!("{}", debug_log);

                        Ok(())
                    }
                    Err(error) => {
                        // Do not throw error
                        // Cancel order could be filled
                        warn!("Can't cancel order: {}. It could be filled", error);
                        // warn!("Cancel Order {:#?}", cancel_order);

                        Ok(())
                    }
                }
            }

            ExchangeName::Huobi => {
                let huobi_account = self.account.huobi.as_ref().unwrap();

                match huobi_account.cancel_order_with_custom_id(&symbol, &custom_order_id) {
                    Ok(_transaction) => {
                        debug!("{}", debug_log);

                        Ok(())
                    }
                    Err(error) => {
                        warn!("Can't cancel order: {}. It could be filled", error);
                        // warn!("Cancel Order {:#?}", cancel_order);

                        Ok(())
                    }
                }
            }

            ExchangeName::BitMEX => {
                let _bitmex_account = self.account.bitmex.as_ref().unwrap();
                Ok(())
            }
            ExchangeName::StubExchange => {
                // Stub exchange
                // Order canceled
                Ok(())
            }
        }
    }

    // Stops gateway and its all dependent threads
    pub fn stop(&self) -> Result<(), &'static str> {
        let gateway_params_lock = self.gateway_params.read().unwrap();

        info!("Gateway {} is stopping", gateway_params_lock.name);

        let gateway_status_lock = self.status.write();

        match gateway_status_lock {
            Ok(mut status) => match *status {
                GatewayStatus::Active => match self.stop_channel.0.send(()) {
                    Ok(_) => {
                        self.stop_channel.0.send(()).unwrap();

                        *status = GatewayStatus::Stopped;

                        info!("Gatewway {} has been stopped", gateway_params_lock.name);
                        Ok(())
                    }
                    Err(_e) => {
                        let error_msg = "Gateway hasn't stopped";
                        println!("{}", error_msg);
                        info!(error_msg);
                        Err(error_msg)
                    }
                },

                GatewayStatus::Stopped => {
                    let error_msg = "Gateway is not running";
                    println!("{}", error_msg);
                    info!(error_msg);
                    Err(error_msg)
                }
            },

            Err(_) => Err("Lock error"),
        }
    }

    // Get info about Gateway
    pub fn info(&self) -> Result<String, &'static str> {
        let gateway_params_lock = self.gateway_params.read().unwrap();
        info!("Getting info for {} Gateway", gateway_params_lock.name);
        Ok(format!(
            r#"Gateway
name: {}
status: {:?}
"#,
            gateway_params_lock.name,
            *self.status.read().unwrap(),
        ))
    }

    pub fn get_gateway_params(&self) -> Result<GatewayParams, &'static str> {
        match self.gateway_params.read() {
            Ok(gateway_params) => Ok(gateway_params.clone()),
            Err(_lock_error) => Err("Gateway params lock error"),
        }
    }

    pub fn get_gateway_name(&self) -> Result<String, &'static str> {
        match self.get_gateway_params() {
            Ok(gateway_params) => Ok(gateway_params.name),
            Err(e) => Err(e),
        }
    }

    // It returns hashmap with gateway name as a key and list of robot names as a value that work with this gateway
    pub fn dependent_robots(robots: Vec<platform::config::Robot>) -> HashMap<String, Vec<String>> {
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for robot in robots {
            let robot_config = RobotParams::from_config(&robot.config_file_path).unwrap();
            let robot_name = robot_config.name;

            for robot_pnl in robot_config.pnl.components {
                let gateway_name = Gateway::extract_gateway_name(&robot_pnl.gateway);

                match dependents.get_mut(&gateway_name) {
                    Some(robots) => {
                        robots.push(robot_name.clone());
                    }
                    None => {
                        dependents.insert(gateway_name, vec![robot_name.clone()]);
                    }
                }
            }
        }
        dependents
    }

    pub fn extract_gateway_name(gateway_indentifier: &str) -> String {
        if gateway_indentifier.contains("::") {
            let gateway_name = gateway_indentifier.split("::").collect::<Vec<&str>>()[0];
            return gateway_name.to_string();
        }
        let mut chars = gateway_indentifier.chars().collect::<Vec<char>>();
        chars[0] = chars[0].to_uppercase().nth(0).unwrap();
        chars.into_iter().collect::<String>()
    }

    // Get status of Gateway
    pub fn status(&self) -> Result<GatewayStatus, &'static str> {
        let gateway_params = self.gateway_params.read().unwrap();
        info!("Getting status of {} Gateway", gateway_params.name);
        match self.status.read() {
            Ok(status) => Ok((*status).clone()),
            Err(_lock_error) => Err("Gateway status lock error"),
        }
    }

    // Set config for Gateway
    pub fn set_config(&self, config_file_path: &str) -> Result<(), &'static str> {
        let gateway_status_lock = self.status.read().unwrap();
        match *gateway_status_lock {
            GatewayStatus::Active => Err("Gateway is Active. Stop it before to set config"),
            GatewayStatus::Stopped => {
                info!("Setting config for Gateway");
                match GatewayConfig::from_file(config_file_path) {
                    Ok(gateway_config) => match GatewayParams::validate_config(&gateway_config) {
                        Ok(_) => {
                            self._set_config(gateway_config);
                            Ok(())
                        }
                        Err(_error) => Err("Config validation error"),
                    },
                    Err(_e) => Err("No gateway config"),
                }
            }
        }
    }

    fn _set_config(&self, gateway_config: GatewayConfig) {
        let mut gateway_params_lock = self.gateway_params.write().unwrap();

        gateway_params_lock.name = gateway_config.gateway_name;

        gateway_params_lock.exchange = ExchangeName::from_str(&gateway_config.exchange).unwrap();

        gateway_params_lock.accounts = gateway_config
            .accounts
            .iter()
            .map(|a| GatewayParamsAccount {
                name: a.name.clone(),
                account_id: a.account_id.clone(),
                api_key: a.api_key.clone(),
                secret_key: a.secret_key.clone(),
            })
            .collect();

        gateway_params_lock.instruments = gateway_config
            .instruments
            .iter()
            .map(|i| Instrument {
                name: i.name.clone(),
                base: i.base.clone(),
                quote: i.quote.clone(),
                lot_size: i.lot_size,
                min_order_size: i.min_order_size,
            })
            .collect();

        gateway_params_lock.fees = gateway_config
            .fees
            .iter()
            .map(|f| Fee {
                account_name: f.account_name.clone(),
                amount_fee: f.amount_fee,
            })
            .collect();

        gateway_params_lock.exchange_time_limit = TimeLimit {
            rpc: gateway_config.limit.rps,
        };
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::gateway::gateway_params::test_utils::GatewayParamsUtils;
    use crate::order_manager::OrderContainer;
    use crate::platform::PlatformConfig;
    use crossbeam::channel::unbounded;
    use std::sync::Arc;

    impl Gateway {
        // Creates Gateway with default parameters with stub channels
        fn new() -> &'static Self {
            // let order_receiver: Receiver<OrderMsg> = unbounded().1;
            // let info_sender: Sender<Info> = unbounded().0;
            // let metadata: tokio::sync::RwLock<Vec<Metadata>> = tokio::sync::RwLock::new(vec![]);
            // let stop_channel: (Sender<()>, Receiver<()>) = bounded(0);

            // Box::leak(Box::new(Gateway {
            //     gateway_params: GatewayParams::default(),
            //     order_receiver,
            //     info_sender,
            //     metadata,
            //     stop_channel,
            // }))

            let gateway_params = GatewayParams::default();

            Gateway::from_params(gateway_params)
        }

        // Creates Gateway with custom parameters with real channels
        fn create(name: &str, order_receiver: Receiver<OrderMsg>) -> &'static Self {
            let info_sender: Sender<GatewayMsg> = unbounded().0;
            let active_order_sender = unbounded().0;

            let metadata: tokio::sync::RwLock<HashMap<String, ExchangeInstrumentInfo>> =
                tokio::sync::RwLock::new(HashMap::new());
            let stop_channel: (Sender<()>, Receiver<()>) = bounded(0);

            Box::leak(Box::new(Gateway {
                gateway_params: Arc::new(RwLock::new(GatewayParams {
                    name: name.to_string(),
                    exchange: ExchangeName::StubExchange,
                    ..GatewayParams::default()
                })),
                status: Arc::new(RwLock::new(GatewayStatus::Stopped)),
                order_containers: Arc::new(RwLock::new(VecDeque::new())),

                orders_receiver: order_receiver,
                info_sender,
                active_order_sender,

                metadata: Arc::new(metadata),
                stop_channel,
                exchange: Arc::new(vec![]),
                account: Accounts::default(),
                websocket: Arc::new(WebSocket::default()),
            }))
        }

        fn from_params(params: GatewayParams) -> &'static Self {
            let (_s, order_receiver): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();
            let (info_sender, _r): (Sender<GatewayMsg>, Receiver<GatewayMsg>) = unbounded();
            let (active_order_sender, _r): (Sender<ActiveOrderMsg>, Receiver<ActiveOrderMsg>) =
                unbounded();

            let metadata: tokio::sync::RwLock<HashMap<String, ExchangeInstrumentInfo>> =
                tokio::sync::RwLock::new(HashMap::new());

            let stop_channel: (Sender<()>, Receiver<()>) = bounded(0);

            let _ = Box::leak(Box::new(_s));
            let _ = Box::leak(Box::new(_r));

            Box::leak(Box::new(Gateway {
                gateway_params: Arc::new(RwLock::new(params.clone())),
                status: Arc::new(RwLock::new(GatewayStatus::Stopped)),
                order_containers: Arc::new(RwLock::new(VecDeque::new())),

                //channels
                orders_receiver: order_receiver,
                info_sender,
                active_order_sender,

                metadata: Arc::new(metadata),
                stop_channel,
                exchange: Arc::new(vec![]),
                account: Accounts::get(&params),
                websocket: Arc::new(WebSocket::default()),
            }))
        }
    }

    #[tokio::test]
    async fn start_gateway() {
        let gateway = Gateway::new();

        assert!(gateway.status().unwrap() == GatewayStatus::Stopped);
        assert!(gateway.start().is_ok());
        assert!(gateway.status().unwrap() == GatewayStatus::Active);
    }

    #[tokio::test]
    async fn stop_gateway() {
        let gateway = Gateway::new();

        assert!(gateway.status().unwrap() == GatewayStatus::Stopped);
        assert!(gateway.start().is_ok());
        assert!(gateway.status().unwrap() == GatewayStatus::Active);
        assert!(gateway.stop().is_ok());
        assert!(gateway.status().unwrap() == GatewayStatus::Stopped);
    }

    #[tokio::test]
    async fn start_gateway_twice() {
        let gateway = Gateway::new();

        assert!(gateway.status().unwrap() == GatewayStatus::Stopped);
        assert!(gateway.start().is_ok());
        assert!(gateway.start().is_err());
        assert!(gateway.status().unwrap() == GatewayStatus::Active);
    }

    #[tokio::test]
    async fn stop_gateway_twice() {
        let gateway = Gateway::new();

        gateway.start().unwrap();
        assert!(gateway.status().unwrap() == GatewayStatus::Active);
        assert!(gateway.stop().is_ok());
        assert!(gateway.status().unwrap() == GatewayStatus::Stopped);
        assert!(gateway.stop().is_err());
        assert!(gateway.status().unwrap() == GatewayStatus::Stopped);
    }

    #[tokio::test]
    async fn stop_gateway_without_start() {
        let gateway = Gateway::new();

        assert!(gateway.status().unwrap() == GatewayStatus::Stopped);
        assert!(gateway.stop().is_err());
        assert!(gateway.status().unwrap() == GatewayStatus::Stopped);
    }

    #[tokio::test]
    async fn receive_order() {
        let (order_sender, order_receiver): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();

        let gateway = Gateway::create("Gateway1", order_receiver);

        order_sender
            .send(OrderMsg::OrderContainers(vec![OrderContainer {
                robot_id: "Robot1".to_string(),
                order: Order::default(),
                metainfo: StrategyParams::Stub,
                created_at: Instant::now(),
            }]))
            .unwrap();

        gateway.receive_order().unwrap();

        assert_eq!(gateway.order_containers.read().unwrap().len(), 1);
    }

    #[test]
    fn extract_gateway_name() {
        assert_eq!(
            "Huobi".to_string(),
            Gateway::extract_gateway_name("Huobi::PROD")
        );

        assert_eq!("Huobi".to_string(), Gateway::extract_gateway_name("Huobi"));

        assert_eq!("Huobi".to_string(), Gateway::extract_gateway_name("huobi"));
    }

    #[test]
    fn dependent_robots() {
        use crate::config::get_config;

        let platform_config: PlatformConfig =
            get_config("test_files/platform_config.toml").unwrap();

        let dependent_robots = Gateway::dependent_robots(platform_config.robots);

        println!("{:?}", dependent_robots);
    }

    #[test]
    fn send_info() {
        let gatewap_params = GatewayParams {
            exchange: ExchangeName::StubExchange,
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        let symbol = "BTCUSDT";

        assert!(gateway
            .send_info(symbol, &ExchangeName::StubExchange)
            .is_ok());
    }

    #[test]
    fn fetch_balance() {
        let gatewap_params = GatewayParams {
            exchange: ExchangeName::StubExchange,
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        assert!(gateway.fetch_balances().is_ok());
    }

    #[test]
    #[ignore]
    // For local testing
    fn fetch_balance_print() {
        let gatewap_params = GatewayParams {
            exchange: ExchangeName::StubExchange,
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        println!("Balances {:?}", gateway.fetch_balances());
    }

    #[test]
    #[ignore]
    // For local testing
    fn fetch_balance_binance() {
        let account = GatewayParamsUtils::binance_test_params();

        let gatewap_params = GatewayParams {
            exchange: ExchangeName::Binance,
            accounts: vec![account],
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        println!("Balances {:?}", gateway.fetch_balances());
    }

    #[test]
    #[ignore]
    // For local testing
    fn fetch_balance_huobi() {
        let account = GatewayParamsUtils::huobi_test_params();

        let gatewap_params = GatewayParams {
            exchange: ExchangeName::Huobi,
            accounts: vec![account],
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        println!("Balances {:?}", gateway.fetch_balances());
    }

    #[tokio::test]
    async fn send_orders() {
        let (order_sender, order_receiver): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();

        let gateway = Gateway::create("Gateway1", order_receiver);

        order_sender
            .send(OrderMsg::OrderContainers(vec![OrderContainer::default()]))
            .unwrap();

        order_sender
            .send(OrderMsg::OrderContainers(vec![OrderContainer::default()]))
            .unwrap();

        gateway.receive_order().unwrap();
        gateway.receive_order().unwrap();

        assert_eq!(gateway.order_containers.read().unwrap().len(), 2);

        gateway.send_order(&ExchangeName::StubExchange).unwrap();
        gateway.send_order(&ExchangeName::StubExchange).unwrap();

        assert_eq!(gateway.order_containers.read().unwrap().len(), 0);
    }

    #[test]
    fn moratorium() {
        let moratorium_order = OrderMsg::OrderContainers(vec![OrderContainer {
            robot_id: "Robot_Huobi_1_BTC".to_string(),
            order: Order::LimitOrder(LimitOrder {
                gateway: "Gateway1".to_string(),
                symbol: "BTCUSDT".to_string(),
                amount: 1.,
                price: 101., // price is more than balance on default account
                order_side: OrderSide::Buy,
                custom_order_id: "".to_string(),
            }),

            metainfo: StrategyParams::Stub,
            created_at: Instant::now(),
        }]);

        let (order_sender, order_receiver): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();

        let gateway = Gateway::create("Gateway1", order_receiver);

        assert_eq!(gateway.order_containers.read().unwrap().len(), 0);

        order_sender.send(moratorium_order).unwrap();

        assert_eq!(gateway.order_containers.read().unwrap().len(), 0);
    }

    #[test]
    fn receive_order_after_moratorium() {
        let moratorium_order = OrderMsg::OrderContainers(vec![OrderContainer {
            robot_id: "Robot_Huobi_1_BTC".to_string(),
            order: Order::LimitOrder(LimitOrder {
                gateway: "Gateway1".to_string(),
                symbol: "BTCUSDT".to_string(),
                amount: 1.,
                price: 151., // price is more than balance on default account
                order_side: OrderSide::Buy,
                custom_order_id: "".to_string(),
            }),

            metainfo: StrategyParams::Stub,
            created_at: Instant::now(),
        }]);

        let (order_sender, order_receiver): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();

        let gateway = Gateway::create("Gateway1", order_receiver);

        assert_eq!(gateway.order_containers.read().unwrap().len(), 0);

        order_sender.send(moratorium_order).unwrap();

        gateway.receive_order().unwrap();

        gateway.send_order(&ExchangeName::StubExchange).unwrap();

        assert_eq!(gateway.order_containers.read().unwrap().len(), 0);

        //Wait more than moratorium time
        thread::sleep(Duration::from_secs(EXCHANGE_MORATORIUM_TIME * 2));

        // Order was received after moratorium
        assert_eq!(gateway.order_containers.read().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn fetch_metadata() {
        let gatewap_params = GatewayParams {
            exchange: ExchangeName::StubExchange,
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        assert!(gateway
            .fetch_metadata(&ExchangeName::StubExchange)
            .await
            .is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    // For local testing
    async fn fetch_metadata_binance() {
        let account = GatewayParamsUtils::binance_test_params();

        let gatewap_params = GatewayParams {
            exchange: ExchangeName::Binance,
            accounts: vec![account],
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        gateway
            .fetch_metadata(&ExchangeName::Binance)
            .await
            .unwrap();

        println!("Metadata: {:?}", gateway.metadata.read().await);
    }
    #[tokio::test]
    #[ignore]
    // For local testing
    async fn fetch_metadata_huobi() {
        let account = GatewayParamsUtils::huobi_test_params();

        let gatewap_params = GatewayParams {
            exchange: ExchangeName::Huobi,
            accounts: vec![account],
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        gateway.fetch_metadata(&ExchangeName::Huobi).await.unwrap();

        println!("Metadata: {:?}", gateway.metadata.read().await);
    }

    #[test]
    fn fetch_depth() {
        let gatewap_params = GatewayParams {
            exchange: ExchangeName::StubExchange,
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        let symbol = "BTCUSDT";

        assert!(gateway
            .fetch_depth(symbol.to_string(), &ExchangeName::StubExchange)
            .is_ok());
    }

    #[test]
    #[ignore]
    // For local testing
    fn fetch_depth_binance() {
        let account = GatewayParamsUtils::binance_test_params();

        let gatewap_params = GatewayParams {
            exchange: ExchangeName::Binance,
            accounts: vec![account],
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        let symbol = "BTCUSDT";

        loop {
            println!(
                "Orderbook {:?}",
                gateway.fetch_depth(symbol.to_string(), &ExchangeName::Binance)
            );
        }
    }

    #[test]
    #[ignore]
    // For local testing
    fn fetch_depth_huobi() {
        let account = GatewayParamsUtils::huobi_test_params();

        let gatewap_params = GatewayParams {
            exchange: ExchangeName::Huobi,
            accounts: vec![account],
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        let symbol = "BTCUSDT";

        println!(
            "Orderbook {:?}",
            gateway.fetch_depth(symbol.to_string(), &ExchangeName::Huobi)
        );
    }

    #[test]
    fn get_gateway_name() {
        let gateway = Gateway::new();
        assert!(gateway.get_gateway_name().is_ok());
    }

    #[test]
    #[ignore]
    // For local testing
    fn get_gateway_name_print() {
        let gateway = Gateway::new();
        println!("Gateway name: {}", gateway.get_gateway_name().unwrap());
    }

    #[test]
    fn load() {
        let (info_sender, _r): (Sender<GatewayMsg>, Receiver<GatewayMsg>) = unbounded();
        let (_s, order_receiver): (Sender<OrderMsg>, Receiver<OrderMsg>) = unbounded();
        let (active_order_sender, _r): (Sender<ActiveOrderMsg>, Receiver<ActiveOrderMsg>) =
            unbounded();

        let config_file_path = "test_files/gateway_config.toml";

        let gateway = Gateway::load(
            config_file_path,
            info_sender,
            order_receiver,
            active_order_sender,
        );
        assert!(gateway.is_ok());
    }

    #[test]
    fn status() {
        let gateway = Gateway::new();
        assert!(gateway.status().is_ok());
    }

    #[test]
    #[ignore]
    // For local testing
    fn status_print() {
        let gateway = Gateway::new();
        println!("Gateway status {}", gateway.status().unwrap());
    }

    #[test]
    fn info() {
        let gateway = Gateway::new();
        assert!(gateway.info().is_ok());
    }

    #[test]
    #[ignore]
    // For local testing
    fn info_print() {
        let gateway = Gateway::new();
        println!("Gateway info: {:?}", gateway.info().unwrap());
    }

    #[test]
    fn cancel_order() {
        let gatewap_params = GatewayParams {
            exchange: ExchangeName::StubExchange,
            ..GatewayParams::default()
        };

        let gateway = Gateway::from_params(gatewap_params);

        assert!(gateway
            .cancel_order(CancelOrder::default(), &ExchangeName::StubExchange)
            .is_ok());
    }

    // #[test]
    // fn check_balance() {
    //     let gatewap_params = GatewayParams {
    //         exchange: ExchangeNameList::StubExchange,
    //         ..GatewayParams::default()
    //     };

    //     let gateway = Gateway::from_params(gatewap_params);

    //     gateway.check_balance(limit_order);
    // }

    #[test]
    #[ignore]
    fn test_ws_huobi() {
        let symbol = "btcusdt";

        let mut huobi_ws = HuobiWS::connect(symbol);

        loop {
            let depth = huobi_ws.get_depth();

            println!("{:?}", depth);
        }
    }
}
