use super::{storage::StorageConnection, Storage};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use strum_macros::EnumString;
use tracing::{debug, info};

const ROBOT_DB_FIELDS: &str = "name, strategy, gateway, instruments, timestamp";

// Stub
#[derive(Debug, Serialize, Deserialize, PartialEq, EnumString)]
pub enum Symbol {
    BTC,
    ETH,
    Stub,
}

#[derive(Debug)]
pub struct RobotDB {
    pub id: i32,
    pub name: String,
    pub strategy: String,
    pub gateway: String,
    //TODO replace to Vec<u8> with bincode
    pub instruments: Vec<u8>,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Instrument {
    pub base: Symbol,
    pub quote: Symbol,
}

impl PartialEq for RobotDB {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.gateway == other.gateway
    }
}

impl Default for RobotDB {
    fn default() -> Self {
        RobotDB {
            id: 0,
            name: "Robot".to_string(),
            strategy: "Demo".to_string(),
            gateway: "Huobi".to_string(),
            instruments: bincode::serialize(&vec![Instrument {
                base: Symbol::BTC,
                quote: Symbol::ETH,
            }])
            .unwrap(),
            timestamp: "2020-11-12 19:37:24.196818324 UTC".to_string(),
        }
    }
}

pub struct RobotStore {
    conn: Connection,
}

impl StorageConnection for RobotStore {
    fn new_connection() -> Self {
        RobotStore {
            conn: Self::connection(),
        }
    }
    fn new_in_memory() -> Self {
        RobotStore {
            conn: Self::connection_in_memory(),
        }
    }
}

impl Storage<RobotDB> for RobotStore {
    fn init(&self) -> Result<usize> {
        info!("Robot Store init");
        let init_robot_store_sql = "CREATE TABLE IF NOT EXISTS robot(
                              id          INTEGER PRIMARY KEY,
                              name        TEXT NOT NULL,
                              strategy    TEXT NOT NULL,
                              gateway     TEXT NOT NULL,
                              instruments BLOB,
                              timestamp   TEXT NOT NULL
                        )";

        debug!("[start] Robot Store: init storage");
        let count = self.conn.execute(init_robot_store_sql, params![])?;
        debug!("[end] Robot Store: init storage");
        Ok(count)
    }

    fn store(&self, robot: &RobotDB) -> Result<usize> {
        info!("Robot Store store");
        let insert_robot_sql = format!(
            "INSERT INTO robot ({}) VALUES (?1, ?2, ?3, ?4, ?5)",
            ROBOT_DB_FIELDS
        );
        debug!("[start] Robot Store: store robot");
        let count = self.conn.execute(
            &insert_robot_sql,
            params![
                robot.name,
                robot.strategy,
                robot.gateway,
                robot.instruments,
                robot.timestamp
            ],
        )?;
        debug!("[end] Robot Store: store robot");
        Ok(count)
    }

    fn select(&self, name: &str) -> Result<Vec<RobotDB>> {
        info!("Robot Store: select robot by name");

        let select_robot_sql = format!("SELECT id, {} FROM robot WHERE name = ?1", ROBOT_DB_FIELDS);
        debug!("[start] Robot Store: select by name");
        let mut stmt = self.conn.prepare(&select_robot_sql)?;
        debug!("[end] Robot Store: select by name");

        let robots = stmt.query_map(params![name], |row| {
            Ok(RobotDB {
                id: row.get(0)?,
                name: row.get(1)?,
                strategy: row.get(2)?,
                gateway: row.get(3)?,
                instruments: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })?;

        Ok(robots.map(|robot| robot.unwrap()).collect())
    }

    fn select_all(&self) -> Result<Vec<RobotDB>> {
        info!("Robot Store: select all robots");

        let select_all_robot_sql = format!("SELECT id, {} FROM robot", ROBOT_DB_FIELDS);
        debug!("[start] Robot Store: select all robots");
        let mut stmt = self.conn.prepare(&select_all_robot_sql)?;
        debug!("[end] Robot Store: select all robots");

        let robots = stmt.query_map(params![], |row| {
            Ok(RobotDB {
                id: row.get(0)?,
                name: row.get(1)?,
                strategy: row.get(2)?,
                gateway: row.get(3)?,
                instruments: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })?;

        Ok(robots.map(|robot| robot.unwrap()).collect())
    }

    fn is_empty(&self) -> bool {
        info!("Robot Store: check whether the storage is empty");
        self.select_all().unwrap().len() == 0
    }

    fn remove(&self, name: &str) -> Result<usize> {
        info!("Robot Store: remove robot by name");
        let remove_robot_sql = "DELETE FROM robot WHERE name = ?1";

        debug!("[start] Robot Store: remove robot by name");
        let count = self.conn.execute(remove_robot_sql, params![name])?;
        debug!("[end] Robot Store: select by name");

        Ok(count)
    }

    fn remove_all(&self) -> Result<usize> {
        info!("Robot Store: remove all robots");
        let remove_all_robots_sql = "DELETE FROM robot";

        debug!("[start] Robot Store: remove all robots");
        let count = self.conn.execute(remove_all_robots_sql, params![])?;
        debug!("[end] Robot Store: remove all robots");

        Ok(count)
    }

    fn load(&self, name: &str) -> Result<Vec<RobotDB>> {
        info!("Robot Store: load robot by name");

        let selected_robot = self.select(name);

        // If found nothing return robot not found
        if let Ok(robot) = &selected_robot {
            if robot.len() == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        }
        // Remove robot from database after select it
        let _ = self.remove(name);

        selected_robot
    }

    fn load_all(&self) -> Result<Vec<RobotDB>> {
        info!("Robot Store load all robots");

        let robots = self.select_all();
        self.remove_all().unwrap();
        robots
    }
}

#[cfg(test)]
mod tests {

    use super::{
        super::storage::StorageConnection, Instrument, RobotDB, RobotStore, Storage, Symbol,
    };
    use bincode;

    fn init_store() -> RobotStore {
        let robot_store = RobotStore::new_in_memory();
        robot_store.init().unwrap();
        robot_store
    }

    fn get_test_robot(
        id: i32,
        name: &str,
        strategy: &str,
        gateway: &str,
        instruments: Vec<Instrument>,
        timestamp: &str,
    ) -> RobotDB {
        RobotDB {
            id: id,
            name: name.to_string(),
            strategy: strategy.to_string(),
            gateway: gateway.to_string(),
            instruments: bincode::serialize(&instruments).unwrap(),
            timestamp: timestamp.to_string(),
        }
    }

    #[test]
    fn test_store_load() {
        let robot_store = init_store();
        let initial_robot: RobotDB = Default::default();

        let count = robot_store.store(&initial_robot).unwrap();
        assert_eq!(count, 1);

        let loaded_robot = robot_store.load_all();
        assert_eq!(initial_robot, *loaded_robot.unwrap().first().unwrap());
    }

    #[test]
    fn test_empty_load() {
        let robot_store = RobotStore::new_in_memory();
        robot_store.init().unwrap();
        let loaded_robot = robot_store.load_all();
        assert!(loaded_robot.unwrap().is_empty());
    }

    #[test]
    #[should_panic]
    fn test_empty_load_without_init() {
        let robot_store = RobotStore::new_in_memory();
        let _ = robot_store.load_all().unwrap();
    }

    #[test]
    fn test_store_multiple_items() {
        let robot_store = init_store();

        let first_robot = Default::default();
        let second_robot = Default::default();

        robot_store.store(&first_robot).unwrap();
        robot_store.store(&second_robot).unwrap();

        let loaded_robots = robot_store.load_all();

        assert_eq!(loaded_robots.unwrap().len(), 2);
    }

    #[test]
    fn test_store_multiple_times() {
        let robot_store = RobotStore::new_in_memory();
        robot_store.init().unwrap();

        let robot = Default::default();

        let _ = robot_store.store(&robot).unwrap();
        let _ = robot_store.store(&robot).unwrap();

        let loaded_robots = robot_store.load_all();

        assert_eq!(loaded_robots.unwrap().len(), 2);
    }

    #[test]
    fn test_table_already_exists() {
        let robot_store = RobotStore::new_in_memory();
        let _ = robot_store.init().unwrap();
        let _ = robot_store.init().unwrap();
    }

    #[test]
    fn test_load_robot_by_name() {
        let robot_store = RobotStore::new_in_memory();
        robot_store.init().unwrap();

        let robot1 = get_test_robot(
            0,
            "Robot1",
            "Demo",
            "Huobi",
            vec![Instrument {
                base: Symbol::BTC,
                quote: Symbol::ETH,
            }],
            &RobotDB::default().timestamp,
        );

        let robot2 = get_test_robot(
            1,
            "Robot2",
            "Demo",
            "Huobi",
            vec![Instrument {
                base: Symbol::BTC,
                quote: Symbol::ETH,
            }],
            &RobotDB::default().timestamp,
        );

        robot_store.store(&robot1).unwrap();
        robot_store.store(&robot2).unwrap();

        let loaded_robot = robot_store.load("Robot1");

        assert_eq!(robot1, *loaded_robot.unwrap().first().unwrap());
    }

    #[test]
    fn test_load_count() {
        let robot_store = RobotStore::new_in_memory();
        robot_store.init().unwrap();

        let robot = Default::default();

        robot_store.store(&robot).unwrap();

        let count_before = robot_store.select(&robot.name);

        // assert robot count is 1
        assert_eq!(count_before.unwrap().len(), 1);

        let _ = robot_store.load("Robot");
        let count_after = robot_store.select(&robot.name);

        // assert robot count is 0
        assert_eq!(count_after.unwrap().len(), 0);
    }

    #[test]
    fn test_remove_robot() {
        let robot_store = RobotStore::new_in_memory();
        robot_store.init().unwrap();

        let robot = Default::default();

        robot_store.store(&robot).unwrap();

        let _ = robot_store.remove(&robot.name);

        assert!(robot_store.is_empty());
    }

    #[test]
    fn test_is_empty() {
        let robot_store = RobotStore::new_in_memory();
        robot_store.init().unwrap();
        assert!(robot_store.is_empty());
    }

    #[test]
    fn test_remove_all_robots() {
        let robot_store = RobotStore::new_in_memory();
        robot_store.init().unwrap();

        let robot1 = get_test_robot(
            0,
            "Robot1",
            "Demo",
            "Huobi",
            vec![Instrument {
                base: Symbol::BTC,
                quote: Symbol::ETH,
            }],
            &RobotDB::default().timestamp,
        );

        let robot2 = get_test_robot(
            1,
            "Robot2",
            "Demo",
            "Huobi",
            vec![Instrument {
                base: Symbol::BTC,
                quote: Symbol::ETH,
            }],
            &RobotDB::default().timestamp,
        );

        robot_store.store(&robot1).unwrap();
        robot_store.store(&robot2).unwrap();

        let _ = robot_store.remove_all();
        assert!(robot_store.is_empty());
    }

    #[test]
    fn test_remove_all_robots_empty() {
        let robot_store = RobotStore::new_in_memory();
        robot_store.init().unwrap();

        let _ = robot_store.remove_all();
        assert!(robot_store.is_empty());
    }

    #[test]
    fn test_count_load_all() {
        let robot_store = init_store();

        let robot1 = get_test_robot(
            0,
            "Robot1",
            "Demo",
            "Huobi",
            vec![Instrument {
                base: Symbol::BTC,
                quote: Symbol::ETH,
            }],
            &RobotDB::default().timestamp,
        );

        let robot2 = get_test_robot(
            1,
            "Robot2",
            "Demo",
            "Huobi",
            vec![Instrument {
                base: Symbol::BTC,
                quote: Symbol::ETH,
            }],
            &RobotDB::default().timestamp,
        );

        let _ = robot_store.store(&robot1).unwrap();
        let _ = robot_store.store(&robot2).unwrap();

        let _ = robot_store.load_all().unwrap();

        assert!(robot_store.is_empty());
    }
}
