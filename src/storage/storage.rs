use super::config::DB_FILE_PATH;
use rusqlite::{Connection, Result};

pub trait StorageConnection {
    fn connection() -> Connection {
        match Connection::open(DB_FILE_PATH) {
            Ok(connection) => connection,
            Err(e) => {
                eprintln!("DB connection error: {}", e);
                // Handle error if needed
                panic!()
            }
        }
    }

    // This method is for run tests only
    fn connection_in_memory() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn new_connection() -> Self;

    fn new_in_memory() -> Self;
}

pub trait Storage<T> {
    fn init(&self) -> Result<usize>;

    fn store(&self, item: &T) -> Result<usize>;

    fn select(&self, name: &str) -> Result<Vec<T>>;

    fn select_all(&self) -> Result<Vec<T>>;

    fn is_empty(&self) -> bool;

    fn remove(&self, name: &str) -> Result<usize>;

    fn remove_all(&self) -> Result<usize>;

    fn load(&self, name: &str) -> Result<Vec<T>>;

    fn load_all(&self) -> Result<Vec<T>>;
}
