use super::{storage::StorageConnection, Storage};
use rusqlite::{Connection, Result};

#[derive(Debug)]
pub struct OrderBookDB {
    pub id: i32,
    pub gateway_name: String,
    pub instrument_name: String,
    pub symbol: String,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct Order {
    pub price_whole_part: i32,
    pub price_fraction_part: i32,
    pub quantity_whole_part: i32,
    pub quantity_fraction_part: i32,
    pub order_timestamp: String,
}

pub struct OrderBookStore {
    conn: Connection,
}

impl StorageConnection for OrderBookStore {
    fn new_connection() -> Self {
        OrderBookStore {
            conn: Self::connection(),
        }
    }

    fn new_in_memory() -> Self {
        OrderBookStore {
            conn: Self::connection_in_memory(),
        }
    }
}

impl Storage<OrderBookDB> for OrderBookStore {
    fn init(&self) -> Result<usize> {
        todo!()
    }

    fn store(&self, _item: &OrderBookDB) -> Result<usize> {
        todo!()
    }

    fn select(&self, _name: &str) -> Result<Vec<OrderBookDB>> {
        todo!()
    }

    fn select_all(&self) -> Result<Vec<OrderBookDB>> {
        todo!()
    }

    fn is_empty(&self) -> bool {
        todo!()
    }

    fn remove(&self, _name: &str) -> Result<usize> {
        todo!()
    }

    fn remove_all(&self) -> Result<usize> {
        todo!()
    }

    fn load(&self, _name: &str) -> Result<Vec<OrderBookDB>> {
        todo!()
    }

    fn load_all(&self) -> Result<Vec<OrderBookDB>> {
        todo!()
    }
}
