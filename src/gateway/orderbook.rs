use super::{Depth, Ticker};
use std::{cmp::Ordering, collections::HashMap};
use std::{iter::Rev, mem};
use tracing::{error, info};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct Price((u64, i16, i8));

impl Price {
    fn new(price: f64) -> Self {
        Self::get_price_obj(price)
    }

    fn get_price_obj(price: f64) -> Self {
        Price(Self::decode(price))
    }

    fn get_price_float(price: &Self) -> f64 {
        Self::encode(price.0)
    }

    // Gets price as f64 number
    pub fn as_f64(&self) -> f64 {
        Self::get_price_float(self)
    }

    fn encode((mantissa, exponent, sign): (u64, i16, i8)) -> f64 {
        (sign as f64) * (mantissa as f64) * (2f64.powf(exponent as f64))
    }

    fn decode(val: f64) -> (u64, i16, i8) {
        let bits: u64 = unsafe { mem::transmute(val) };
        let sign: i8 = if bits >> 63 == 0 { 1 } else { -1 };
        let mut exponent: i16 = ((bits >> 52) & 0x7ff) as i16;
        let mantissa = if exponent == 0 {
            (bits & 0xfffffffffffff) << 1
        } else {
            (bits & 0xfffffffffffff) | 0x10000000000000
        };

        exponent -= 1023 + 52;
        (mantissa, exponent, sign)
    }
}

// impl Ord for Price {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         Self::get_price_float(self).cmp(Self::get_price_float(other))
//     }
// }

impl PartialOrd for Price {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Self::get_price_float(self).partial_cmp(&Self::get_price_float(other))
    }
}

#[derive(Clone, Debug)]
pub struct OrderBook {
    pub instrument_name: String,

    gateway_name: String,

    // price, amount
    bids: HashMap<Price, f64>,
    asks: HashMap<Price, f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Volume {
    // total volume size for all exchanges
    pub sum: f64,
    // separated volume size by exchange
    pub exchange_volume: HashMap<String, f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CumulativeOrderBook {
    pub instrument_name: String,

    // price, amount
    bids: HashMap<Price, Volume>,
    asks: HashMap<Price, Volume>,
}

enum Side {
    Bid,
    Ask,
}

impl OrderBook {
    fn new(instrument_name: &str, gateway_name: &str) -> Self {
        OrderBook {
            instrument_name: instrument_name.to_string(),
            gateway_name: gateway_name.to_string(),
            bids: HashMap::new(),
            asks: HashMap::new(),
        }
    }

    pub fn from_depth(depth: Depth, symbol: &str, gateway_name: &str) -> Self {
        let mut bids = HashMap::new();
        let mut asks = HashMap::new();

        for _bids in depth.bids {
            bids.insert(Price::new(_bids.price), _bids.qty);
        }

        for _asks in depth.asks {
            asks.insert(Price::new(_asks.price), _asks.qty);
        }

        OrderBook {
            instrument_name: symbol.to_string(),
            gateway_name: gateway_name.to_string(),
            bids,
            asks,
        }
    }

    // Calculate cumulative OrderBook
    pub fn cumulative_book(books: Vec<Self>) -> Result<Self, &'static str> {
        fn cumulate(tickers_group: &Vec<Vec<Ticker>>) -> Result<HashMap<Price, f64>, &'static str> {
            let mut side = HashMap::new();

            tickers_group.iter().for_each(|tickers| {
                tickers.iter().for_each(|ticker| {
                    let price = Price::get_price_obj(ticker.price);

                    *side.entry(price).or_insert(0.) += ticker.qty;
                });
            });

            Ok(side)
        }

        match books.first() {
            Some(ob) => {
                let instrument = ob.instrument_name.clone();

                Ok(OrderBook {
                    instrument_name: instrument,
                    gateway_name: "GatewayStub".to_string(),
                    bids: cumulate(
                        &books
                            .iter()
                            .map(|book| book.to_depth().bids)
                            .collect::<Vec<Vec<Ticker>>>(),
                    )?,
                    asks: cumulate(
                        &books
                            .iter()
                            .map(|book| book.to_depth().asks)
                            .collect::<Vec<Vec<Ticker>>>(),
                    )?,
                })
            }
            None => Err("No orderbooks"),
        }
    }

    /// Returns bid prices iterator in descending order
    ///
    /// #Example:
    ///
    /// ```
    /// let order_book = OrderBook::default();
    /// let mut bids_iter = order_book.bids_iter();
    ///
    /// bids_iter.next(); // 999.
    /// bids_iter.next(); // 998.
    /// bids_iter.next(); // 997.
    /// ```
    pub fn bids_iter(&self) -> Rev<std::vec::IntoIter<&Price>> {
        let mut prices = self.bids.iter().map(|item| item.0).collect::<Vec<&Price>>();

        prices.sort_by(|p1, p2| p1.partial_cmp(p2).unwrap());

        prices.into_iter().rev()
    }

    /// Returns bid price and size iterator in descending order
    /// (price, size)
    pub fn bids_volume_iter(&self) -> std::vec::IntoIter<(f64, f64)> {
        let mut bids = self.bids.clone().into_iter().collect::<Vec<(Price, f64)>>();

        bids.sort_by(|b1, b2| b1.0.partial_cmp(&b2.0).unwrap());

        bids.iter()
            .map(|(p, v)| (p.as_f64(), *v))
            .rev()
            .collect::<Vec<(f64, f64)>>()
            .into_iter()
    }

    /// Returns ask prices iterator in ascending order
    ///
    /// #Example:
    ///
    /// ```
    /// let order_book = OrderBook::default();
    /// let mut asks_iter = order_book.asks_iter();
    ///
    /// asks_iter.next(); // 1001.
    /// asks_iter.next(); // 1002.
    /// asks_iter.next(); // 1003.
    /// ```
    pub fn asks_iter(&self) -> std::vec::IntoIter<&Price> {
        let mut prices = self.asks.iter().map(|item| item.0).collect::<Vec<&Price>>();

        prices.sort_by(|p1, p2| p1.partial_cmp(p2).unwrap());

        prices.into_iter()
    }

    /// Returns ask price and size iterator in ascending order
    /// (price, size)
    pub fn asks_volume_iter(&self) -> std::vec::IntoIter<(f64, f64)> {
        let mut asks = self.asks.clone().into_iter().collect::<Vec<(Price, f64)>>();

        asks.sort_by(|b1, b2| b1.0.partial_cmp(&b2.0).unwrap());

        asks.iter()
            .map(|(p, v)| (p.as_f64(), *v))
            .collect::<Vec<(f64, f64)>>()
            .into_iter()
    }

    pub fn to_depth(&self) -> Depth {
        let mut _bids = Vec::new();
        let mut _asks = Vec::new();

        for bid in self.bids.iter() {
            _bids.push(Ticker {
                price: Price::get_price_float(bid.0),
                qty: *bid.1,
            });
        }

        for ask in self.asks.iter() {
            _asks.push(Ticker {
                price: Price::get_price_float(ask.0),
                qty: *ask.1,
            });
        }

        Depth {
            exchange: self.gateway_name.clone(),
            bids: _bids,
            asks: _asks,
        }
    }

    // Get amount by price for Bid
    fn bids_get(&self, price: f64) -> Option<&f64> {
        let price_obj = Price::get_price_obj(price);
        self.bids.get(&price_obj)
    }

    // Get amount by price for Ask
    fn asks_get(&self, price: f64) -> Option<&f64> {
        let price_obj = Price::get_price_obj(price);
        self.bids.get(&price_obj)
    }

    // Add new order to Order Book
    fn add_order(&mut self, price: f64, amount: f64, side: Side) -> Result<(), &'static str> {
        let price_obj = Price::get_price_obj(price);
        match side {
            Side::Bid => {
                *self.bids.entry(price_obj).or_insert(0.) += amount;
                // info!("New bid order was added");
            }
            Side::Ask => {
                *self.asks.entry(price_obj).or_insert(0.) += amount;
                // info!("New ask order was added");
            }
        }
        Ok(())
    }

    fn remove_order(&mut self, _price: f64, _side: Side) -> Result<(), &'static str> {
        // implement it later if needed
        todo!()
    }

    // Remove entery level from Order Book
    fn remove_level(&mut self, price: f64, side: Side) -> Result<f64, &'static str> {
        let price_obj = Price::get_price_obj(price);
        match side {
            Side::Bid => match self.bids.remove(&price_obj) {
                Some(e) => {
                    info!("Bid level {} was removed from Order Book", price);
                    Ok(e)
                }
                None => {
                    error!("Bid level not found");
                    Err("Bid level not found")
                }
            },
            Side::Ask => match self.asks.remove(&price_obj) {
                Some(e) => {
                    info!("Ask level {} was removed from Order Book", price);
                    Ok(e)
                }
                None => {
                    error!("Ask level not found");
                    Err("Ask level not found")
                }
            },
        }
    }

    fn weighted_prices(order_book: &OrderBook) -> [f64; 2] {
        fn weighted(tickers: Vec<Ticker>) -> f64 {
            tickers
                .iter()
                .map(|ticker| ticker.price * ticker.qty)
                .sum::<f64>()
                / tickers.iter().map(|ticker| ticker.qty).sum::<f64>()
        }

        let depth = order_book.to_depth();

        [weighted(depth.bids), weighted(depth.asks)]
    }

    // Accepts vector of (price, size)
    pub fn weighted(side: Vec<(f64, f64)>) -> f64 {
        side.iter().map(|(price, size)| price * size).sum::<f64>()
            / side.iter().map(|(_price, size)| size).sum::<f64>()
    }

    pub fn from_vec(
        instrument_name: &str,
        gateway_name: &str,
        bids: Vec<[f64; 2]>,
        asks: Vec<[f64; 2]>,
    ) -> Self {
        let mut order_book = OrderBook {
            instrument_name: instrument_name.to_string(),
            gateway_name: gateway_name.to_string(),
            bids: HashMap::new(),
            asks: HashMap::new(),
        };

        bids.iter()
            .for_each(|bid| order_book.add_order(bid[0], bid[1], Side::Bid).unwrap());

        asks.iter()
            .for_each(|ask| order_book.add_order(ask[0], ask[1], Side::Ask).unwrap());

        order_book
    }
}

// Implement default OrderBook for making tests
impl Default for OrderBook {
    fn default() -> Self {
        let bid_values = vec![
            [995., 10.],
            [996., 40.],
            [997., 20.],
            [998., 40.],
            [999., 10.],
        ];
        let ask_values = vec![
            [1001., 10.],
            [1002., 40.],
            [1003., 20.],
            [1004., 40.],
            [1005., 10.],
        ];

        OrderBook::from_vec("BTCUSDT", "GatewayStub", bid_values, ask_values)
    }
}

// Utility functions for tests
impl OrderBook {
    pub fn stub() -> Self {
        let bid_values = vec![
            [1.1, 10.1],
            [1.1, 20.1],
            [1.2, 9.9],
            [1.2, 9.8],
            [0.9, 10.2],
        ];
        let ask_values = vec![
            [1.1, 10.1],
            [1.1, 20.1],
            [1.2, 9.9],
            [1.2, 9.8],
            [0.9, 10.2],
        ];

        OrderBook::stub_from_vec(bid_values, ask_values)
    }

    pub fn stub_from_vec(bids: Vec<[f64; 2]>, asks: Vec<[f64; 2]>) -> Self {
        Self::from_vec("BTCUSDT", "GatewayStub", bids, asks)
    }
}

impl PartialEq for OrderBook {
    fn eq(&self, other: &Self) -> bool {
        self.instrument_name == other.instrument_name
            && self.gateway_name == other.gateway_name
            && self.bids == other.bids
            && self.asks == other.asks
    }
}

impl CumulativeOrderBook {
    pub fn new(orderbooks: Vec<OrderBook>) -> Result<Self, &'static str> {
        match orderbooks.first() {
            Some(ob) => {
                let instrument = ob.instrument_name.clone();

                let mut bids: HashMap<Price, Volume> = HashMap::new();
                let mut asks: HashMap<Price, Volume> = HashMap::new();

                for orderbook in orderbooks {
                    for bid in orderbook.bids {
                        match bids.get_mut(&bid.0) {
                            Some(volume) => {
                                volume.sum += bid.1;
                                volume
                                    .exchange_volume
                                    .insert(orderbook.gateway_name.clone(), bid.1);
                            }
                            None => {
                                let mut exchange = HashMap::new();
                                exchange.insert(orderbook.gateway_name.clone(), bid.1);

                                bids.insert(
                                    bid.0,
                                    Volume {
                                        sum: bid.1,
                                        exchange_volume: exchange,
                                    },
                                );
                            }
                        }
                    }

                    for ask in orderbook.asks {
                        match asks.get_mut(&ask.0) {
                            Some(volume) => {
                                volume.sum += ask.1;
                                volume
                                    .exchange_volume
                                    .insert(orderbook.gateway_name.clone(), ask.1);
                            }
                            None => {
                                let mut exchange = HashMap::new();
                                exchange.insert(orderbook.gateway_name.clone(), ask.1);

                                asks.insert(
                                    ask.0,
                                    Volume {
                                        sum: ask.1,
                                        exchange_volume: exchange,
                                    },
                                );
                            }
                        }
                    }
                }

                Ok(CumulativeOrderBook {
                    instrument_name: instrument,
                    bids: bids,
                    asks: asks,
                })
            }
            None => Err("No orderbooks"),
        }
    }

    /// Returns bid price and Volume iterator in descending order
    /// (price, Volume)
    pub fn bids_volume_iter(&self) -> std::vec::IntoIter<(f64, Volume)> {
        let mut bids = self
            .bids
            .clone()
            .into_iter()
            .collect::<Vec<(Price, Volume)>>();

        bids.sort_by(|b1, b2| b1.0.partial_cmp(&b2.0).unwrap());

        bids.iter()
            .map(|(p, v)| (p.as_f64(), v.clone()))
            .rev()
            .collect::<Vec<(f64, Volume)>>()
            .into_iter()
    }

    /// Returns ask price and Volume iterator in ascending order
    /// (price, size)
    pub fn asks_volume_iter(&self) -> std::vec::IntoIter<(f64, Volume)> {
        let mut asks = self
            .asks
            .clone()
            .into_iter()
            .collect::<Vec<(Price, Volume)>>();

        asks.sort_by(|b1, b2| b1.0.partial_cmp(&b2.0).unwrap());

        asks.iter()
            .map(|(p, v)| (p.as_f64(), v.clone()))
            .collect::<Vec<(f64, Volume)>>()
            .into_iter()
    }
}

#[cfg(test)]
mod tests {

    use std::iter::FromIterator;

    use super::*;

    fn is_equal(amount: f64, other_amount: f64) -> bool {
        (amount - other_amount).abs() < 1.0e-8
    }

    #[test]
    fn price_convert() {
        let value = 5.055;
        let price = Price::new(value);

        assert!(is_equal(Price::get_price_float(&price), value));
    }

    #[test]
    fn price_equal() {
        let price1 = Price::new(1.0000001);
        let price2 = Price::new(1.0000001);
        assert_eq!(price1, price2)
    }

    #[test]
    fn price_not_equal() {
        let price1 = Price::new(1.0000001);
        let price2 = Price::new(1.0000002);
        assert_ne!(price1, price2)
    }

    #[test]
    fn sort_price() {
        let mut prices = vec![
            Price::new(1.),
            Price::new(1.1),
            Price::new(1.9),
            Price::new(1.5),
            Price::new(10.),
            Price::new(1.05),
            Price::new(1.01),
            Price::new(10.),
            Price::new(0.01),
            Price::new(0.0000001),
        ];

        prices.sort_by(|p1, p2| p1.partial_cmp(p2).unwrap());

        let sorted_prices = vec![
            Price::new(0.0000001),
            Price::new(0.01),
            Price::new(1.),
            Price::new(1.01),
            Price::new(1.05),
            Price::new(1.1),
            Price::new(1.5),
            Price::new(1.9),
            Price::new(10.),
            Price::new(10.),
        ];

        assert_eq!(prices, sorted_prices);
    }

    #[test]
    fn cumulative_book() {
        let book_one = OrderBook::stub_from_vec(
            vec![[996., 20.], [997., 10.], [998., 20.], [999., 10.]],
            vec![[1002., 20.], [1003., 10.], [1004., 20.], [1005., 10.]],
        );

        let book_two = OrderBook::stub_from_vec(
            vec![[995., 10.], [996., 20.], [997., 10.], [998., 20.]],
            vec![[1001., 10.], [1002., 20.], [1003., 10.], [1004., 20.]],
        );

        let result_book = OrderBook::stub_from_vec(
            vec![
                [995., 10.],
                [996., 40.],
                [997., 20.],
                [998., 40.],
                [999., 10.],
            ],
            vec![
                [1001., 10.],
                [1002., 40.],
                [1003., 20.],
                [1004., 40.],
                [1005., 10.],
            ],
        );

        let cumulative_book = OrderBook::cumulative_book(vec![book_one, book_two]);

        assert_eq!(result_book, cumulative_book.unwrap());
    }

    #[test]
    fn cumulative_book_empty() {
        let cumulative_book = OrderBook::cumulative_book(vec![]);

        // No orderbooks
        assert!(cumulative_book.is_err());
    }

    #[test]
    fn cumulative_book_empty_one() {}

    #[test]
    fn cumulative_book_empty_both() {
        assert_eq!(
            OrderBook::stub_from_vec(vec![], vec![]),
            OrderBook::cumulative_book(vec![
                OrderBook::stub_from_vec(vec![], vec![]),
                OrderBook::stub_from_vec(vec![], vec![])
            ])
            .unwrap()
        );
    }

    #[test]
    fn add_order() {
        let mut order_book = OrderBook::stub();
        assert!(order_book.add_order(1.1, 1.1, Side::Bid).is_ok());
    }

    #[test]
    fn add_order_compare() {
        let mut order_book = OrderBook::stub();
        order_book.add_order(1.1, 10.1, Side::Bid).unwrap();

        assert!(is_equal(*order_book.bids_get(1.1).unwrap(), 40.3));
    }

    #[test]
    #[ignore]
    fn remove_order() {
        let mut order_book = OrderBook::stub();
        assert!(order_book.remove_order(1.1, Side::Bid).is_ok());
    }

    #[test]
    #[ignore]
    fn remove_order_compare() {
        let mut order_book = OrderBook::stub();
        order_book.remove_order(1.1, Side::Bid).unwrap();
        //TODO compare
    }

    #[test]
    fn remove_level() {
        let mut order_book = OrderBook::stub();
        assert!(order_book.remove_level(1.1, Side::Bid).is_ok());
    }

    #[test]
    fn remove_level_compare() {
        let mut order_book = OrderBook::stub();
        order_book.remove_level(1.1, Side::Bid).unwrap();
        assert_eq!(order_book.bids.len(), 2);
    }

    #[test]
    fn get_bids() {
        let order_book = OrderBook::stub();

        assert!(is_equal(*order_book.bids_get(1.2).unwrap(), 19.7));
    }

    #[test]
    fn get_asks() {
        let order_book = OrderBook::stub();

        assert!(is_equal(*order_book.asks_get(1.2).unwrap(), 19.7));
    }

    #[test]
    fn bids_iter() {
        let order_book = OrderBook::default();
        let mut bids_iter = order_book.bids_iter();

        assert_eq!(bids_iter.next().unwrap().as_f64(), 999.);
        assert_eq!(bids_iter.next().unwrap().as_f64(), 998.);
        assert_eq!(bids_iter.next().unwrap().as_f64(), 997.);
    }

    #[test]
    fn bids_volume_iter() {
        let order_book = OrderBook::default();
        let mut bids_volume = order_book.bids_volume_iter();

        assert_eq!(bids_volume.next(), Some((999.0, 10.0)));
        assert_eq!(bids_volume.next(), Some((998.0, 40.0)));
        assert_eq!(bids_volume.next(), Some((997.0, 20.0)));
    }

    #[test]
    fn asks_iter() {
        let order_book = OrderBook::default();
        let mut askss_iter = order_book.asks_iter();

        assert_eq!(askss_iter.next().unwrap().as_f64(), 1001.);
        assert_eq!(askss_iter.next().unwrap().as_f64(), 1002.);
        assert_eq!(askss_iter.next().unwrap().as_f64(), 1003.);
    }

    #[test]
    fn asks_volume_iter() {
        let order_book = OrderBook::default();
        let mut asks_volume = order_book.asks_volume_iter();

        assert_eq!(asks_volume.next(), Some((1001.0, 10.0)));
        assert_eq!(asks_volume.next(), Some((1002.0, 40.0)));
        assert_eq!(asks_volume.next(), Some((1003.0, 20.0)));
    }

    #[test]
    fn weighted_prices() {
        let bids = vec![
            [995., 10.],
            [996., 20.],
            [997., 10.],
            [998., 20.],
            [999., 10.],
        ];
        let asks = vec![
            [1001., 10.],
            [1002., 20.],
            [1003., 10.],
            [1004., 20.],
            [1005., 10.],
        ];

        let order_book = OrderBook::stub_from_vec(bids, asks);

        let weighted_prices = OrderBook::weighted_prices(&order_book);

        assert_eq!(weighted_prices, [997.0, 1003.0])
    }

    #[test]
    fn weighted() {
        let side = vec![
            (995., 10.),
            (996., 20.),
            (997., 10.),
            (998., 20.),
            (999., 10.),
        ];

        let weighted = OrderBook::weighted(side);

        assert_eq!(weighted, 997.0);
    }

    #[test]
    fn cumulative_order_book() {
        use std::array::IntoIter;
        let binance_book = OrderBook::from_vec(
            "BTCUSDT",
            "Binance",
            vec![[996., 20.], [997., 10.], [998., 20.], [999., 10.]],
            vec![[1002., 20.], [1003., 10.], [1004., 20.], [1005., 10.]],
        );

        let huobi_book = OrderBook::from_vec(
            "BTCUSDT",
            "Huobi",
            vec![[995., 10.], [996., 20.], [997., 10.], [998., 20.]],
            vec![[1001., 10.], [1002., 20.], [1003., 10.], [1004., 20.]],
        );

        let cumulative = CumulativeOrderBook::new(vec![binance_book, huobi_book]).unwrap();

        let result_ob = CumulativeOrderBook {
            instrument_name: "BTCUSDT".to_string(),
            bids: HashMap::<_, _>::from_iter(IntoIter::new([
                (
                    Price::get_price_obj(996.),
                    Volume {
                        sum: 40.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([
                            ("Huobi".to_string(), 20.0),
                            ("Binance".to_string(), 20.0),
                        ])),
                    },
                ),
                (
                    Price::get_price_obj(997.),
                    Volume {
                        sum: 20.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([
                            ("Huobi".to_string(), 10.0),
                            ("Binance".to_string(), 10.0),
                        ])),
                    },
                ),
                (
                    Price::get_price_obj(998.),
                    Volume {
                        sum: 40.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([
                            ("Huobi".to_string(), 20.0),
                            ("Binance".to_string(), 20.0),
                        ])),
                    },
                ),
                (
                    Price::get_price_obj(995.),
                    Volume {
                        sum: 10.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([(
                            "Huobi".to_string(),
                            10.0,
                        )])),
                    },
                ),
                (
                    Price::get_price_obj(999.),
                    Volume {
                        sum: 10.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([(
                            "Binance".to_string(),
                            10.0,
                        )])),
                    },
                ),
            ])),

            asks: HashMap::<_, _>::from_iter(IntoIter::new([
                (
                    Price::get_price_obj(1002.),
                    Volume {
                        sum: 40.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([
                            ("Huobi".to_string(), 20.0),
                            ("Binance".to_string(), 20.0),
                        ])),
                    },
                ),
                (
                    Price::get_price_obj(1003.),
                    Volume {
                        sum: 20.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([
                            ("Huobi".to_string(), 10.0),
                            ("Binance".to_string(), 10.0),
                        ])),
                    },
                ),
                (
                    Price::get_price_obj(1004.),
                    Volume {
                        sum: 40.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([
                            ("Huobi".to_string(), 20.0),
                            ("Binance".to_string(), 20.0),
                        ])),
                    },
                ),
                (
                    Price::get_price_obj(1001.),
                    Volume {
                        sum: 10.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([(
                            "Huobi".to_string(),
                            10.0,
                        )])),
                    },
                ),
                (
                    Price::get_price_obj(1005.),
                    Volume {
                        sum: 10.0,
                        exchange_volume: HashMap::<_, _>::from_iter(IntoIter::new([(
                            "Binance".to_string(),
                            10.0,
                        )])),
                    },
                ),
            ])),
        };

        assert_eq!(cumulative, result_ob);

        // let mut bids_volume = cumulative.bids_volume_iter();

        // println!("{:#?}", bids_volume.next());
        // println!("{:#?}", bids_volume.next());
    }
}
