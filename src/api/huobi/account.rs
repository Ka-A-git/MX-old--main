use super::{
    client::Client,
    models::{
        APIResult, AccountsResult, Asset, BalanceResult, CancelOrderResult,
        CancelOrderWithIdResult, ExchangeApiResult, OpenOrdersResult, PlaceOrderResult,
        TradeHistoryResult, Transaction,
    },
};
use serde_json;
use std::collections::BTreeMap;
use tracing::{debug, error};

#[derive(Clone)]
pub struct Account {
    // api_key: Option<String>,
    // secret_key: Option<String>,
    pub account_id: String,
    pub client: Client,
}
impl Account {
    pub fn new(account_id: &str, api_key: Option<String>, secret_key: Option<String>) -> Self {
        Account {
            account_id: account_id.to_string(),
            client: Client::new(&api_key.unwrap(), &secret_key.unwrap()),
        }
    }

    pub fn get_open_orders(
        &self,
        symbol: &str,
        // side: String,
        // from: String,
        // direct: String,
        // size: u32,
    ) -> APIResult<OpenOrdersResult> {
        let mut params: BTreeMap<String, String> = BTreeMap::new();

        params.insert("account-id".into(), self.account_id.clone().into());
        params.insert("symbol".into(), symbol.to_lowercase().into());

        let data = self.client.get_signed("/v1/order/openOrders", params)?;

        debug!("[Huobi] Get open orders {:?}", data);

        let open_orders: OpenOrdersResult = serde_json::from_str(data.as_str())?;

        Ok(open_orders)
    }

    pub fn limit_buy(
        &self,
        symbol: &str,
        amount: f64,
        price: f64,
        client_order_id: Option<String>,
    ) -> ExchangeApiResult<Transaction> {
        match self._place_order(
            &self.account_id,
            amount,
            Some(price),
            symbol,
            "buy-limit",
            client_order_id,
        ) {
            Ok(placed_order) => {
                debug!(
                    "[Huobi API] Limit buy: symbol {}, amount {}, price {}",
                    symbol, amount, price
                );

                Ok(Transaction {
                    symbol: symbol.to_string(),
                    order_id: placed_order.data.parse().unwrap(),
                })
            }
            Err(e) => {
                error!("Huobi limit buy error {:?}", e);

                println!("error {:?}", e);
                Err("Huobi Limit buy Error")
            }
        }
    }

    pub fn limit_sell(
        &self,
        symbol: &str,
        amount: f64,
        price: f64,
        client_order_id: Option<String>,
    ) -> ExchangeApiResult<Transaction> {
        match self._place_order(
            &self.account_id,
            amount,
            Some(price),
            symbol,
            "sell-limit",
            client_order_id,
        ) {
            Ok(placed_order) => {
                debug!(
                    "[Huobi API] Limit sell: symbol {}, amount {}, price {}",
                    symbol, amount, price
                );

                Ok(Transaction {
                    symbol: symbol.to_string(),
                    order_id: placed_order.data.parse().unwrap(),
                })
            }
            Err(_) => Err("Huobi Limit sell Error"),
        }
    }

    pub fn market_buy(&self, symbol: &str, amount: f64) -> ExchangeApiResult<Transaction> {
        match self._place_order(&self.account_id, amount, None, symbol, "buy-market", None) {
            Ok(placed_order) => {
                debug!(
                    "[Huobi Huobi] Market buy: symbol {}, amount {}",
                    symbol, amount,
                );

                Ok(Transaction {
                    symbol: symbol.to_string(),
                    order_id: placed_order.data.parse().unwrap(),
                })
            }
            Err(_) => Err("Huobi Market buy Error"),
        }
    }

    pub fn market_sell(&self, symbol: &str, amount: f64) -> ExchangeApiResult<Transaction> {
        match self._place_order(&self.account_id, amount, None, symbol, "sell-market", None) {
            Ok(placed_order) => {
                debug!(
                    "[Huobi API] Market sell: symbol {}, amount {}",
                    symbol, amount,
                );

                Ok(Transaction {
                    symbol: symbol.to_string(),
                    order_id: placed_order.data.parse().unwrap(),
                })
            }
            Err(_) => Err("Huobi Market sell Error"),
        }
    }

    fn _place_order(
        &self,
        account_id: &str,
        amount: f64,
        price: Option<f64>,
        symbol: &str,
        type_: &str,
        client_order_id: Option<String>,
    ) -> APIResult<PlaceOrderResult> {
        let params: BTreeMap<String, String> = BTreeMap::new();
        let mut body: BTreeMap<String, String> = BTreeMap::new();

        body.insert("account-id".into(), account_id.into());
        body.insert("amount".into(), amount.to_string());
        body.insert("price".into(), price.unwrap_or(0.).to_string());
        body.insert("source".into(), "api".into());
        body.insert("symbol".into(), symbol.to_lowercase().into());
        body.insert("type".into(), type_.into());

        if let Some(id) = client_order_id {
            body.insert("client-order-id".into(), id.into());
        }

        let data = self
            .client
            .post_signed("/v1/order/orders/place", params, &body);

        // debug!("[Huobi] Place order result: {:?} ", data?);

        let order: PlaceOrderResult = serde_json::from_str(data?.as_str())?;

        Ok(order)
    }

    pub fn get_accounts(&self) -> APIResult<AccountsResult> {
        let params: BTreeMap<String, String> = BTreeMap::new();

        let data = self.client.get_signed("/v1/account/accounts", params)?;

        debug!("[Huobi] Get accounts result: {:?} ", data);

        let accounts: AccountsResult = serde_json::from_str(data.as_str())?;

        Ok(accounts)
    }

    pub fn cancel_order(&self, symbol: &str, order_id: u64) -> ExchangeApiResult<Transaction> {
        let params: BTreeMap<String, String> = BTreeMap::new();
        let mut body: BTreeMap<String, String> = BTreeMap::new();

        body.insert("order-id".into(), order_id.to_string());

        let endpoint = format!("/v1/order/orders/{}/submitcancel", order_id);

        match self.client.post_signed(&endpoint, params, &body) {
            Ok(data) => {
                let cancel_order: CancelOrderResult = serde_json::from_str(data.as_str()).unwrap();

                debug!(
                    "[Huobi] Order was canceled: symbol {}, {}",
                    symbol, order_id
                );

                Ok(Transaction {
                    symbol: symbol.to_string(),
                    order_id: cancel_order.data.parse().unwrap(),
                })
            }
            Err(_e) => Err("Huobi Request error"),
        }
    }

    pub fn cancel_order_with_custom_id(
        &self,
        symbol: &str,
        custom_order_id: &str,
    ) -> ExchangeApiResult<Transaction> {
        let params: BTreeMap<String, String> = BTreeMap::new();
        let mut body: BTreeMap<String, String> = BTreeMap::new();

        body.insert("client-order-id".into(), custom_order_id.into());

        let endpoint = "/v1/order/orders/submitCancelClientOrder";

        match self.client.post_signed(endpoint, params, &body) {
            Ok(data) => {
                let _cancel_order: CancelOrderWithIdResult =
                    serde_json::from_str(data.as_str()).unwrap();

                debug!(
                    "[Huobi] Order was canceled: symbol {}, {}",
                    symbol, custom_order_id
                );

                Ok(Transaction {
                    symbol: symbol.to_string(),
                    // order_id: cancel_order.data.parse().unwrap(),
                    // We don't know order id
                    order_id: 0,
                })
            }
            Err(_e) => Err("Huobi Request error"),
        }
    }

    pub fn get_all_balances(&self) -> APIResult<BalanceResult> {
        let params: BTreeMap<String, String> = BTreeMap::new();

        let endpoint = format!("/v1/account/accounts/{}/balance", self.account_id);

        let data = self.client.get_signed(&endpoint, params)?;

        debug!("[Huobi] Get balance result: {:?} ", data);

        let balances: BalanceResult = serde_json::from_str(data.as_str())?;

        Ok(balances)
    }

    pub fn get_balance(&self, symbol: &str) -> ExchangeApiResult<Asset> {
        let balances = self.get_all_balances().unwrap();

        let balance = balances
            .data
            .list
            .into_iter()
            // asset type may be both "trade" and "frozen"
            // symbol should be lowercase
            .filter(|asset| asset.trade_type == "trade" && asset.currency == symbol.to_lowercase())
            .nth(0)
            .unwrap();

        Ok(balance)
    }

    pub fn trade_history(&self, symbol: &str) -> APIResult<TradeHistoryResult> {
        let mut params: BTreeMap<String, String> = BTreeMap::new();

        params.insert("symbol".into(), symbol.to_lowercase().into());

        let data = self.client.get_signed("/v1/order/history", params)?;

        debug!("[Huobi] Get trade history {:?}", data);

        let trade_history: TradeHistoryResult = serde_json::from_str(data.as_str())?;

        Ok(trade_history)
    }

    pub fn trade_history_all(&self) -> APIResult<TradeHistoryResult> {
        let params: BTreeMap<String, String> = BTreeMap::new();

        let data = self.client.get_signed("/v1/order/history", params)?;

        debug!("[Huobi] Get all trade history {:?}", data);

        let trade_history: TradeHistoryResult = serde_json::from_str(data.as_str())?;

        Ok(trade_history)
    }
}
