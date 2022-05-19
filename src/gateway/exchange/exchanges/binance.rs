use crate::gateway::exchange::{ExchangeAction, ExchangeApiResult, PlatformTransaction};
use crate::gateway::gateway::ExchangeInstrumentInfo;
use crate::gateway::{self, GatewayParamsAccount, Instrument};
use binance::userstream::UserStream;
use binance::websockets::{
    WebSockets as BinanceWebSockets, WebsocketEvent as BinanceWebsocketEvent,
};
use binance::{
    account::Account,
    api,
    general::General,
    model::{ExchangeInformation, Filters, OrderBook},
    websockets::*,
};

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use tracing::{debug, error, info, warn};

pub struct Binance {
    binance_account: Account,
}

impl Binance {
    pub fn metadata() -> Option<ExchangeInformation> {
        let market: General = api::Binance::new(None, None);

        match market.exchange_info() {
            Ok(exchange_info) => Some(exchange_info),
            Err(e) => {
                error!("Binance exchange error: {}", e);
                None
            }
        }
    }

    pub fn depth_ws<Handler>(handler: Handler, symbols: Vec<&str>)
    where
        Handler: FnMut(binance::websockets::WebsocketEvent) -> Result<(), binance::errors::Error>,
    {
        let _symbols: Vec<_> = symbols.into_iter().map(String::from).collect();

        let mut endpoints: Vec<String> = Vec::new();

        for symbol in _symbols.iter() {
            endpoints.push(format!("{}@depth5@100ms", symbol.to_lowercase()));
        }

        let keep_running = AtomicBool::new(true);
        let mut web_socket: WebSockets<'_> = WebSockets::new(handler);

        web_socket.connect_multiple_streams(&endpoints).unwrap(); // check error

        if let Err(e) = web_socket.event_loop(&keep_running) {
            println!("Error: {:?}", e);
        }
        web_socket.disconnect().unwrap();
    }

    pub fn user_stream_ws<Handler>(config_account: &GatewayParamsAccount, handler: Handler)
    where
        Handler: FnMut(BinanceWebsocketEvent) -> Result<(), binance::errors::Error>,
    {
        let keep_running = AtomicBool::new(true);

        let user_stream: UserStream =
            binance::api::Binance::new(Some(config_account.api_key.clone()), None);

        if let Ok(answer) = user_stream.start() {
            let listen_key = answer.listen_key;

            let mut web_socket: BinanceWebSockets = BinanceWebSockets::new(handler);

            web_socket.connect(&listen_key).unwrap(); // check error
            if let Err(e) = web_socket.event_loop(&keep_running) {
                match e {
                    err => {
                        error!("Error: {:?}", err);
                    }
                }
            }
        } else {
            error!("Not able to start an User Stream (Check your API_KEY)");
        }
    }

    pub fn price_precision(filters: Vec<Filters>) -> u8 {
        Self::get_precision(&Self::get_price_tick_size(filters).unwrap()) as u8
    }

    fn get_precision(tick_size: &str) -> usize {
        let float_part = tick_size.split(".").skip(1).next().unwrap();

        for (index, char) in float_part.char_indices() {
            if char != '0' {
                return index + 1;
            }
        }
        return 0;
    }

    fn get_price_tick_size(filters: Vec<Filters>) -> Option<String> {
        for filter in filters {
            if let Filters::PriceFilter {
                min_price: _,
                max_price: _,
                tick_size,
            } = filter
            {
                return Some(tick_size.clone());
            }
        }

        None
    }

    pub fn get_depth(order_book: &OrderBook) -> gateway::Depth {
        let depth = gateway::Depth {
            exchange: "Binance".to_string(),
            bids: order_book
                .bids
                .iter()
                .map(|t| gateway::Ticker {
                    price: t.price,
                    qty: t.qty,
                })
                .collect(),
            asks: order_book
                .asks
                .iter()
                .map(|t| gateway::Ticker {
                    price: t.price,
                    qty: t.qty,
                })
                .collect(),
        };

        depth
    }
}

impl ExchangeAction for Binance {
    fn inti(&self) {
        todo!()
    }

    fn fetch_metadata(&self) -> Vec<ExchangeInstrumentInfo> {
        let mut instruments_info = Vec::new();

        let binance_metadata = Self::metadata().unwrap();

        for symbol_info in binance_metadata.symbols {
            instruments_info.push(ExchangeInstrumentInfo {
                base: symbol_info.base_asset,
                quote: symbol_info.quote_asset,
                symbol: symbol_info.symbol,
                precision: Self::price_precision(symbol_info.filters),
            });
        }

        instruments_info
    }

    fn fetch_depth(&self, _symbol: &str) -> Result<gateway::Depth, &'static str> {
        todo!()
    }

    fn fetch_balances(
        &self,
        instruments: Vec<Instrument>,
    ) -> Result<HashMap<String, f64>, &'static str> {
        let mut instrument_balances = HashMap::new();

        for instrument in instruments {
            let balance_base = self.binance_account.get_balance(&instrument.base);
            let balance_quote = self.binance_account.get_balance(&instrument.quote);

            info!("[Gateway] Got balance for Binance account");

            instrument_balances.insert(
                instrument.base.clone(),
                balance_base.unwrap().free.parse().unwrap(),
            );

            instrument_balances.insert(
                instrument.quote.clone(),
                balance_quote.unwrap().free.parse().unwrap(),
            );
        }

        Ok(instrument_balances)
    }

    fn limit_buy(
        &self,
        symbol: &str,
        amount: f64,
        price: f64,
        custom_order_id: Option<String>,
    ) -> ExchangeApiResult<PlatformTransaction> {
        match self.binance_account.custom_order(
            symbol,
            amount,
            price,
            None,
            binance::account::OrderSide::Buy,
            binance::account::OrderType::Limit,
            binance::account::TimeInForce::GTC,
            custom_order_id,
        ) {
            Ok(transaction) => {
                debug!("[Binance] Ok. Limit Buy order was placed");

                Ok(PlatformTransaction {
                    symbol: transaction.symbol,
                    order_id: transaction.order_id,
                })
            }

            Err(error) => {
                error!("Binance Limit Buy error: {}", error);
                Err(Box::leak(Box::new(error)).description())
            }
        }
    }

    fn limit_sell(
        &self,
        symbol: &str,
        amount: f64,
        price: f64,
        custom_order_id: Option<String>,
    ) -> ExchangeApiResult<PlatformTransaction> {
        match self.binance_account.custom_order(
            symbol,
            amount,
            price,
            None,
            binance::account::OrderSide::Sell,
            binance::account::OrderType::Limit,
            binance::account::TimeInForce::GTC,
            custom_order_id,
        ) {
            Ok(transaction) => {
                debug!("[Binance] Ok. Limit Sell order was placed");

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

    fn market_buy(&self, symbol: &str, amount: f64) -> ExchangeApiResult<PlatformTransaction> {
        match self.binance_account.market_buy(symbol, amount) {
            Ok(transaction) => Ok(PlatformTransaction {
                symbol: transaction.symbol,
                order_id: transaction.order_id,
            }),
            Err(error) => Err(Box::leak(Box::new(error)).description()),
        }
    }

    fn market_sell(&self, symbol: &str, amount: f64) -> ExchangeApiResult<PlatformTransaction> {
        match self.binance_account.market_sell(symbol, amount) {
            Ok(transaction) => Ok(PlatformTransaction {
                symbol: transaction.symbol,
                order_id: transaction.order_id,
            }),
            Err(error) => Err(Box::leak(Box::new(error)).description()),
        }
    }

    fn cancel_order(
        &self,
        symbol: &str,
        custom_order_id: &str,
    ) -> ExchangeApiResult<PlatformTransaction> {
        match self
            .binance_account
            .cancel_order_with_client_id(symbol, custom_order_id.to_string())
        {
            Ok(order_canceled) => Ok(PlatformTransaction {
                symbol: order_canceled.symbol,
                order_id: order_canceled.order_id.unwrap_or(0),
            }),

            Err(error) => {
                // Do not throw error
                // Cancel order could be filled

                warn!("Can't cancel order: {}. It could be filled", error);

                Ok(PlatformTransaction {
                    symbol: symbol.to_string(),
                    order_id: 0,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::Binance;

    #[test]
    #[ignore]
    fn metadata() {
        println!("{:?}", Binance::metadata().unwrap().symbols);
    }

    #[test]
    fn get_precision() {
        let tick_size = "0.00000100";

        assert_eq!(6, Binance::get_precision(tick_size));
    }
}
