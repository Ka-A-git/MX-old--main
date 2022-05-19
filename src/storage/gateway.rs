use super::{storage::StorageConnection, Storage};
use rusqlite::{params, Connection, Result};
use tracing::debug;

#[derive(Debug)]
pub struct GatewayDB {
    id: i32,
    name: String,
}

pub struct GatewayStore {
    conn: Connection,
}

impl StorageConnection for GatewayStore {
    fn new_connection() -> Self {
        GatewayStore {
            conn: Self::connection(),
        }
    }

    fn new_in_memory() -> Self {
        GatewayStore {
            conn: Self::connection_in_memory(),
        }
    }
}

impl Storage<GatewayDB> for GatewayStore {
    fn init(&self) -> Result<usize> {
        let init_gateway_store_sql = "CREATE TABLE IF NOT EXISTS gateway(
                                    id       INTEGER PRIMARY KEY,
                                    name     TEXT NOT NULL
                            )";

        debug!("[start] Gateway Store: init storage");
        let count = self.conn.execute(init_gateway_store_sql, params![])?;
        debug!("[end] Gateway Store: init storage");
        Ok(count)
    }

    fn store(&self, _item: &GatewayDB) -> Result<usize> {
        todo!()
    }

    fn select(&self, _name: &str) -> Result<Vec<GatewayDB>> {
        todo!()
    }

    fn select_all(&self) -> Result<Vec<GatewayDB>> {
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

    fn load(&self, _name: &str) -> Result<Vec<GatewayDB>> {
        todo!()
    }

    fn load_all(&self) -> Result<Vec<GatewayDB>> {
        todo!()
    }
}
