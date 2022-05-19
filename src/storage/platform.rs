use super::storage::StorageConnection;
use rusqlite::{params, Connection, Result};
use tracing::{debug, info};

pub struct PlatformDB {
    id: i32,
    //TODO change it
    robots: String,
}

// Storage trait for Platform only
pub trait PlatformStorage<T> {
    fn init(&self) -> Result<usize>;

    fn store(&self, item: &T) -> Result<usize>;

    fn is_empty(&self) -> bool;

    fn remove(&self, name: &str) -> Result<usize>;

    fn load(&self, name: &str) -> Result<Vec<T>>;
}

pub struct PlatformStore {
    conn: Connection,
}

impl StorageConnection for PlatformStore {
    fn new_connection() -> Self {
        PlatformStore {
            conn: Self::connection(),
        }
    }

    fn new_in_memory() -> Self {
        PlatformStore {
            conn: Self::connection_in_memory(),
        }
    }
}

impl PlatformStorage<PlatformDB> for PlatformStore {
    fn init(&self) -> Result<usize> {
        info!("Platform Store init");
        let init_platform_store_sql = "CREATE TABLE IF NOT EXISTS platform(
                            id          INTEGER PRIMARY KEY,
                            robots      TEXT NOT NULL
        )";

        debug!("[start] Platform Store: init storage");
        let count = self.conn.execute(init_platform_store_sql, params![])?;
        debug!("[end] Platform Store: init storage");
        Ok(count)
    }

    fn store(&self, _item: &PlatformDB) -> Result<usize> {
        todo!()
    }

    fn is_empty(&self) -> bool {
        todo!()
    }

    fn remove(&self, _name: &str) -> Result<usize> {
        todo!()
    }

    fn load(&self, _name: &str) -> Result<Vec<PlatformDB>> {
        todo!()
    }
}
