use crate::api::huobi::websocket_account::{
    WebSockets as HuobiWebSockets, WebsocketEvent as HuobiWebsocketEvent,
};
use crate::api::{
    self,
    huobi::{models::ResultSymbol, Account, HuobiApi},
};
use crate::gateway::{
    self,
    exchange::{ExchangeAction, ExchangeApiResult, PlatformTransaction},
    gateway::ExchangeInstrumentInfo,
    Depth, GatewayParamsAccount,
};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use tracing::{debug, error, info, warn};

pub struct Huobi {
    huobi_account: Account,
}

impl Huobi {
    pub async fn metadata() -> Option<ResultSymbol> {
        match HuobiApi::symbols().await {
            Ok(symbols) => Some(symbols),

            Err(e) => {
                error!("Huobi exchange error: {}", e);

                None
            }
        }
    }

    pub fn get_depth(huobi_depth: &api::huobi::websocket_data::Depth) -> gateway::Depth {
        let depth = gateway::Depth {
            exchange: huobi_depth.exchange.clone(),
            bids: huobi_depth
                .bids
                .iter()
                .map(|ticker| gateway::Ticker {
                    price: ticker.price,
                    qty: ticker.qty,
                })
                .collect(),
            asks: huobi_depth
                .asks
                .iter()
                .map(|ticker| gateway::Ticker {
                    price: ticker.price,
                    qty: ticker.qty,
                })
                .collect(),
        };

        depth
    }

    pub fn user_stream_ws<Handler>(
        config_account: &GatewayParamsAccount,
        handler: Handler,
        symbols: Vec<&str>,
    ) where
        Handler: FnMut(HuobiWebsocketEvent) -> Result<(), Box<dyn std::error::Error>>,
    {
        let keep_running = AtomicBool::new(true);

        let accountws: String = "/ws/v2".to_string();

        let mut websocket: HuobiWebSockets = HuobiWebSockets::new(handler);

        websocket
            .connect_auth(
                &accountws,
                symbols,
                vec![],
                &config_account.api_key,
                &config_account.secret_key,
            )
            .unwrap();

        if let Err(e) = websocket.event_loop(&keep_running) {
            match e {
                err => {
                    println!("Error: {}", err);
                }
            }
        }
    }
}

impl ExchangeAction for Huobi {
    fn inti(&self) {
        todo!()
    }

    fn fetch_metadata(&self) -> Vec<ExchangeInstrumentInfo> {
        // let mut instruments_info = Vec::new();

        // let huobi_metadata = Self::metadata().await.unwrap();
        // for symbol_info in huobi_metadata.data {
        //     instruments_info.push(ExchangeInstrumentInfo {
        //         base: symbol_info.base,
        //         quote: symbol_info.quote,
        //         symbol: symbol_info.symbol,
        //         precision: symbol_info.price_precision,
        //     });
        // }

        // instruments_info
        todo!()
    }

    fn fetch_depth(&self, _symbol: &str) -> Result<Depth, &'static str> {
        todo!()
    }

    fn fetch_balances(
        &self,
        instruments: Vec<gateway::Instrument>,
    ) -> Result<HashMap<String, f64>, &'static str> {
        let mut instrument_balances = HashMap::new();
        for instrument in instruments {
            let balance_base = self
                .huobi_account
                .get_balance(&instrument.base)
                .unwrap()
                .balance;

            let balance_quote = self
                .huobi_account
                .get_balance(&instrument.quote)
                .unwrap()
                .balance;

            info!("[Gateway] Got balance for Huobi account");

            instrument_balances.insert(instrument.base.clone(), balance_base);
            instrument_balances.insert(instrument.quote.clone(), balance_quote);
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
        match self
            .huobi_account
            .limit_buy(symbol, amount, price, custom_order_id)
        {
            Ok(transaction) => {
                debug!("[Huobi] Ok. Limit Buy order was placed");

                Ok(PlatformTransaction {
                    symbol: transaction.symbol,
                    order_id: transaction.order_id,
                })
            }
            Err(error) => {
                error!("Huobi Limit Buy error: {}", error);
                Err(error)
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
        match self
            .huobi_account
            .limit_sell(symbol, amount, price, custom_order_id)
        {
            Ok(transaction) => {
                debug!("[Huobi] Ok. Limit Sell order was placed");
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

    fn market_buy(&self, symbol: &str, amount: f64) -> ExchangeApiResult<PlatformTransaction> {
        match self.huobi_account.market_buy(symbol, amount) {
            Ok(transaction) => Ok(PlatformTransaction {
                symbol: transaction.symbol,
                order_id: transaction.order_id,
            }),
            Err(error) => Err(error),
        }
    }

    fn market_sell(&self, symbol: &str, amount: f64) -> ExchangeApiResult<PlatformTransaction> {
        match self.huobi_account.market_sell(symbol, amount) {
            Ok(transaction) => Ok(PlatformTransaction {
                symbol: transaction.symbol,
                order_id: transaction.order_id,
            }),
            Err(error) => Err(error),
        }
    }

    fn cancel_order(
        &self,
        symbol: &str,
        custom_order_id: &str,
    ) -> ExchangeApiResult<PlatformTransaction> {
        match self
            .huobi_account
            .cancel_order_with_custom_id(symbol, custom_order_id)
        {
            Ok(transaction) => Ok(PlatformTransaction {
                symbol: transaction.symbol,
                order_id: transaction.order_id,
            }),
            Err(error) => {
                // Do not throw error
                // Cancel order could be filled

                warn!("Can't cancel order: {}. It could be filled", error);
                // warn!("Cancel Order {:#?}", cancel_order);

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

    use super::Huobi;

    #[tokio::test]
    #[ignore]
    async fn metadata_huobi() {
        println!("{:#?}", Huobi::metadata().await.unwrap());
    }
}
