// Helper module for doing admin actions on exchange

#[cfg(test)]
mod tests {

    use crate::api::huobi::models::EventType;
    use crate::api::huobi::websocket_account::WebsocketEvent as HuobiWebsocketEvent;
    use crate::gateway::exchange::account::Accounts;
    use crate::gateway::gateway_params::test_utils::GatewayParamsUtils;
    use binance::websockets::WebsocketEvent as BinanceWebsocketEvent;
    use std::sync::atomic::AtomicBool;
    use std::thread;
    use std::time::Duration;

    #[test]
    #[ignore]
    // For local testing
    fn limit_buy_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();
        let binance_account = Accounts::binance(&params_account);

        // Symbol should be uppercase
        let symbol = "BTCUSDT";

        // Min valid MIN_NOTIONAL
        // MIN_NOTIONAL error: price * quantity is too low to be a valid order for the symbol.
        let amount = 0.35;

        // Binance API throws an error if price too high or too low
        let price = 30_000.;

        binance_account.limit_buy(symbol, amount, price).unwrap();
    }

    #[test]
    #[ignore]
    // For local testing
    fn limit_buy_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();
        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "BTCUSDT";
        let amount = 1.;
        let price = 9000.;

        huobi_account
            .limit_buy(symbol, amount, price, None)
            .unwrap();
    }

    #[test]
    #[ignore]
    // For local testing
    fn limit_sell_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();
        let binance_account = Accounts::binance(&params_account);

        let symbol = "BTCUSDT";
        let amount = 1.;
        let price = 1_000_000.;

        binance_account.limit_sell(symbol, amount, price).unwrap();
    }

    #[test]
    #[ignore]
    // For local testing
    fn limit_sell_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();
        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "btcusdt";
        let amount = 1.;
        let price = 1_000_000.;

        huobi_account
            .limit_sell(symbol, amount, price, None)
            .unwrap();
    }

    #[test]
    #[ignore]
    // For local testing
    fn market_buy_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();
        let binance_account = Accounts::binance(&params_account);

        let symbol = "BTCUSDT";

        // Amount in btc
        let amount = 0.01;

        match binance_account.market_buy(symbol, amount) {
            Ok(t) => println!("Transactin {:?}", t),
            Err(e) => println!("Error {:?}", e),
        }
    }

    #[test]
    #[ignore]
    // For local testing
    fn market_buy_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();
        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "BTCUSDT";

        // usdt amount
        let amount = 150.0;

        match huobi_account.market_buy(symbol, amount) {
            Ok(t) => println!("Transaction {:?}", t),
            Err(e) => println!("Error {}", e),
        };
    }

    #[test]
    #[ignore]
    // For local testing
    fn market_sell_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();
        let binance_account = Accounts::binance(&params_account);

        let symbol = "BTCUSDT";
        let amount = 0.0005;

        match binance_account.market_sell(symbol, amount) {
            Ok(tr) => println!("Transaction {:?}", tr),
            Err(e) => println!("Error {}", e),
        }
    }

    #[test]
    #[ignore]
    // For local testing
    fn market_sell_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();
        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "btcusdt";
        let amount = 0.003;

        huobi_account.market_sell(symbol, amount).unwrap();
    }

    #[test]
    #[ignore]
    // For local testing
    fn cancel_order_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();

        let binance_account = Accounts::binance(&params_account);

        let symbol = "BTCUSDT";
        let amount = 0.001;
        let price = 30_000.;

        thread::sleep(Duration::from_secs(5));

        match binance_account.limit_buy(symbol, amount, price) {
            Ok(trx) => {
                println!("trx {:?}", trx);

                let symbol = trx.symbol.as_str();
                let order_id = trx.order_id;
                thread::sleep(Duration::from_secs(5));

                match binance_account.cancel_order(symbol, order_id) {
                    Ok(order_canceled) => println!("canceled {:?}", order_canceled),
                    Err(e) => println!("error {:?}", e),
                }
            }
            Err(e) => println!("{:}", e),
        };
    }

    #[test]
    #[ignore]
    // For local testing
    fn cancel_order_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();

        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "btcusdt";
        let amount = 1.;
        let price = 10.;

        thread::sleep(Duration::from_secs(5));

        match huobi_account.limit_buy(symbol, amount, price, None) {
            Ok(trx) => {
                println!("trx {:?}", trx);

                let symbol = trx.symbol.as_str();
                let order_id = trx.order_id;

                // wait 5 seconds
                thread::sleep(Duration::from_secs(5));

                match huobi_account.cancel_order(symbol, order_id) {
                    Ok(order_canceled) => println!("canceled {:?}", order_canceled),
                    Err(e) => println!("error {:?}", e),
                }
            }
            Err(e) => println!("{:}", e),
        };
    }

    #[test]
    #[ignore]
    // For local testing
    fn cancel_order_by_id_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();

        let binance_account = Accounts::binance(&params_account);

        let symbol = "BTCUSDT";
        let order_id = 6339611169;

        match binance_account.cancel_order(symbol, order_id) {
            Ok(order_canceled) => println!("canceled {:?}", order_canceled),
            Err(e) => println!("error {:?}", e),
        }
    }

    #[test]
    #[ignore]
    // For local testing
    fn cancel_order_by_id_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();

        let huobi_account = Accounts::huobi(&params_account);

        // change order_id
        let order_id = 292644691913752;

        match huobi_account.cancel_order("", order_id) {
            Ok(order_canceled) => println!("canceled {:?}", order_canceled),
            Err(e) => println!("error {:?}", e),
        }
    }

    // Test open and close order with custom ID for Binance
    #[test]
    #[ignore]
    // For local testing
    fn cancel_order_with_custom_id_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();

        let binance_account = Accounts::binance(&params_account);

        let symbol = "BTCUSDT";
        let amount = 0.001;
        let price = 30_000.;
        let custom_order_id = "custom_order_id123";

        thread::sleep(Duration::from_secs(5));

        match binance_account.custom_order(
            symbol,
            amount,
            price,
            None,
            binance::account::OrderSide::Buy,
            binance::account::OrderType::Limit,
            binance::account::TimeInForce::GTC,
            Some(custom_order_id.to_string().clone()),
        ) {
            Ok(trx) => {
                println!("trx {:?}", trx);

                let symbol = trx.symbol.as_str();

                thread::sleep(Duration::from_secs(5));

                match binance_account
                    .cancel_order_with_client_id(symbol, custom_order_id.to_string())
                {
                    Ok(order_canceled) => println!("canceled {:?}", order_canceled),
                    Err(e) => println!("error {:?}", e),
                }
            }
            Err(e) => println!("{:}", e),
        };
    }

    // Test open and close order with custom ID for Huobi
    #[test]
    #[ignore]
    // For local testing
    fn cancel_order_with_custom_id_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();

        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "btcusdt";
        let amount = 1.;
        let price = 10.;
        let custom_order_id = "custom_order_id123";

        thread::sleep(Duration::from_secs(5));

        match huobi_account.limit_buy(symbol, amount, price, Some(custom_order_id.to_string())) {
            Ok(trx) => {
                println!("trx {:?}", trx);

                let symbol = trx.symbol.as_str();
                // let order_id = trx.order_id;

                // wait 5 seconds
                thread::sleep(Duration::from_secs(5));

                match huobi_account.cancel_order_with_custom_id(symbol, custom_order_id) {
                    Ok(order_canceled) => println!("canceled {:?}", order_canceled),
                    Err(e) => println!("error {:?}", e),
                }
            }
            Err(e) => println!("{:}", e),
        };
    }

    #[test]
    #[ignore]
    // For local testing
    fn get_open_orders_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();

        let binance_account = Accounts::binance(&params_account);

        // let symbol = "BTCUSDT";
        // let open_orders = binance_account.get_open_orders(symbol);

        let open_orders = binance_account.get_all_open_orders();

        println!("Open orders {:#?}", open_orders);
    }
    #[test]
    #[ignore]
    // For local testing
    fn get_open_orders_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();

        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "btcusdt";

        let open_orders = huobi_account.get_open_orders(symbol);

        println!("Open orders {:#?}", open_orders);
    }

    #[test]
    #[ignore]
    // For local testing
    fn cancel_open_orders_by_symbol_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();

        let binance_account = Accounts::binance(&params_account);

        let symbol = "BTCUSDT";

        let open_orders = binance_account.get_open_orders(symbol);

        for order in open_orders.unwrap() {
            match binance_account.cancel_order(symbol, order.order_id) {
                Ok(order_canceled) => println!("canceled {:?}", order_canceled),
                Err(e) => println!("error {:?}", e),
            }
        }
    }

    #[test]
    #[ignore]
    // For local testing
    fn cancel_open_orders_by_symbol_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();

        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "btcusdt";

        let open_orders = huobi_account.get_open_orders(symbol);

        for order in open_orders.unwrap().data {
            let order_id = order.id;

            match huobi_account.cancel_order("", order_id) {
                Ok(order_canceled) => println!("canceled {:?}", order_canceled),
                Err(e) => println!("error {:?}", e),
            }
        }
    }

    #[test]
    #[ignore]
    // For local testing
    fn get_accounts_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();

        let huobi_account = Accounts::huobi(&params_account);

        let accounts = huobi_account.get_accounts();

        println!("Accounts {:#?}", accounts);
    }

    #[test]
    #[ignore]
    // For local testing
    fn get_balance_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();
        let binance_account = Accounts::binance(&params_account);

        // Low letters do not work
        let btc = "BTC";
        let usdt = "USDT";

        let btc_balance = binance_account.get_balance(btc).unwrap();
        let usdt_balance = binance_account.get_balance(usdt).unwrap();

        println!("BTC balance: {:?}", btc_balance);
        println!("USDT balance: {:?}", usdt_balance);
    }

    #[test]
    #[ignore]
    // For local testing
    fn get_all_balances_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();
        let huobi_account = Accounts::huobi(&params_account);

        let balances = huobi_account.get_all_balances().unwrap();
        println!("Balances: {:#?}", balances);
    }

    #[test]
    #[ignore]
    // For local testing
    fn get_balance_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();
        let huobi_account = Accounts::huobi(&params_account);

        let btc = "btc";
        let usdt = "usdt";

        let btc_asset = huobi_account.get_balance(btc).unwrap();
        let usdt_asset = huobi_account.get_balance(usdt).unwrap();

        println!("BTC balance: {:?}", btc_asset);
        println!("USDT balance: {:?}", usdt_asset);
    }

    #[test]
    #[ignore]
    // For local testing
    fn trade_history_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();
        let binance_account = Accounts::binance(&params_account);

        let symbol = "BTCUSDT";

        let trade_history = binance_account.trade_history(symbol).unwrap();

        println!("Trade history: {:#?}", trade_history);
    }

    #[test]
    #[ignore]
    // For local testing
    fn trade_history_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();
        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "BTCUSDT";

        let trade_history = huobi_account.trade_history(symbol).unwrap();

        println!("Trade history: {:#?}", trade_history);
    }

    #[test]
    #[ignore]
    // For local testing
    fn trade_history_all_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();
        let huobi_account = Accounts::huobi(&params_account);

        let trade_history = huobi_account.trade_history_all().unwrap();

        println!("Trade history: {:?}", trade_history);
    }

    #[test]
    #[ignore]
    // For local testing
    fn order_websocket_binance() {
        let params_account = GatewayParamsUtils::binance_test_params();

        let handler = |event: BinanceWebsocketEvent| {
            match event {
                BinanceWebsocketEvent::OrderTrade(trade) => {
                    println!(
                        "Symbol: {}, Side: {}, Price: {}, Execution Type: {}",
                        trade.symbol, trade.side, trade.price, trade.execution_type
                    );
                }
                _ => {}
            }
            Ok(())
        };

        Accounts::binance_ws(&params_account, handler);
    }

    #[test]
    #[ignore]
    // For local testing
    fn order_websocket_huobi() {
        let params_account = GatewayParamsUtils::huobi_test_params();

        let handler = |event: HuobiWebsocketEvent| {
            match event {
                HuobiWebsocketEvent::OrderUpdate(order_subscription) => {
                    match order_subscription.data {
                        EventType::Creation(order) => {
                            println!("Create order {:?}", order);
                        }
                        EventType::Cancellation(order) => {
                            println!("Cancel order {:?}", order);
                        }
                        EventType::Trade(_) => {}
                    };
                }
            }

            Ok(())
        };

        Accounts::huobi_ws(&params_account, handler, vec!["BTCUSDT"]);
    }

    #[test]
    #[ignore]
    fn fetch_depth_binance_ws() {
        use binance::websockets::*;
        use std::sync::atomic::AtomicBool;

        let symbols: Vec<_> = vec!["btcusdt"].into_iter().map(String::from).collect();

        let mut endpoints: Vec<String> = Vec::new();

        for symbol in symbols.iter() {
            endpoints.push(format!("{}@depth@100ms", symbol.to_lowercase()));
        }

        let keep_running = AtomicBool::new(true);
        let mut web_socket: WebSockets<'_> = WebSockets::new(|event: WebsocketEvent| {
            if let WebsocketEvent::DepthOrderBook(depth_order_book) = event {
                println!("{:?}", depth_order_book);
            }

            Ok(())
        });

        web_socket.connect_multiple_streams(&endpoints).unwrap(); // check error
        if let Err(e) = web_socket.event_loop(&keep_running) {
            println!("Error: {:?}", e);
        }
        web_socket.disconnect().unwrap();
    }

    #[test]
    fn test_price() {
        let price1 = 36207.160005000005;
        let price2 = 36207.149995;

        assert_eq!("36207.16", format!("{:.2}", price1));
        assert_eq!("36207.15", format!("{:.2}", price2));
        // println!("{}", format!("{:.2}", price2).parse::<f64>().unwrap());
    }

    #[test]
    #[ignore]
    // For local testing
    fn test_request_time_binance() {
        use std::time::Instant;

        let params_account = GatewayParamsUtils::binance_test_params();

        let binance_account = Accounts::binance(&params_account);

        let symbol = "BTCUSDT";
        let amount = 0.001;
        let price = 30_000.;
        let custom_order_id = "custom_order_id123";

        let order_time = Instant::now();

        match binance_account.custom_order(
            symbol,
            amount,
            price,
            None,
            binance::account::OrderSide::Buy,
            binance::account::OrderType::Limit,
            binance::account::TimeInForce::GTC,
            Some(custom_order_id.to_string()),
        ) {
            Ok(_trx) => {
                println!("Order time {:?}", order_time.elapsed());

                let cancel_time = Instant::now();

                match binance_account
                    .cancel_order_with_client_id(symbol, custom_order_id.to_string())
                {
                    Ok(_order_canceled) => println!("Cancel time {:?}", cancel_time.elapsed()),

                    Err(e) => println!("error {:?}", e),
                };
            }
            Err(e) => println!("{:}", e),
        };
    }

    #[test]
    #[ignore]
    // For local testing
    fn test_request_time_huobi() {
        use std::time::Instant;

        let params_account = GatewayParamsUtils::huobi_test_params();

        let huobi_account = Accounts::huobi(&params_account);

        let symbol = "btcusdt";
        let amount = 1.;
        let price = 10.;
        let custom_order_id = "custom_order_id123";

        let order_time = Instant::now();

        match huobi_account.limit_buy(symbol, amount, price, Some(custom_order_id.to_string())) {
            Ok(_trx) => {
                println!("Order time {:?}", order_time.elapsed());

                let cancel_time = Instant::now();

                match huobi_account.cancel_order_with_custom_id(symbol, custom_order_id) {
                    Ok(_order_canceled) => println!("Cancel time {:?}", cancel_time.elapsed()),

                    Err(e) => println!("error {:?}", e),
                }
            }
            Err(e) => println!("{:}", e),
        };
    }

    #[test]
    #[ignore]
    // For local testing
    fn balance_ws_binance() {
        use binance::api::*;
        use binance::userstream::*;
        use binance::websockets::*;

        let params_account = GatewayParamsUtils::binance_test_params();

        let api_key_user = Some(params_account.api_key.into());
        let keep_running = AtomicBool::new(true);
        let user_stream: UserStream = Binance::new(api_key_user, None);

        if let Ok(answer) = user_stream.start() {
            let listen_key = answer.listen_key;

            let mut web_socket: WebSockets = WebSockets::new(|event: WebsocketEvent| {
                match event {
                    WebsocketEvent::AccountUpdate(account_update) => {
                        for balance in &account_update.balance {
                            println!(
                                "Asset: {}, free: {}, locked: {}",
                                balance.asset, balance.free, balance.locked
                            );
                        }
                    }
                    WebsocketEvent::OrderTrade(trade) => {
                        println!(
                            "Symbol: {}, Side: {}, Price: {}, Execution Type: {}",
                            trade.symbol, trade.side, trade.price, trade.execution_type
                        );
                    }
                    _ => {}
                };
                Ok(())
            });

            web_socket.connect(&listen_key).unwrap(); // check error
            if let Err(e) = web_socket.event_loop(&keep_running) {
                match e {
                    err => {
                        println!("Error: {:?}", err);
                    }
                }
            }
        } else {
            println!("Not able to start an User Stream (Check your API_KEY)");
        }
    }
}
