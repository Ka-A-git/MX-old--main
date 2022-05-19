use super::{
    robot::RobotStatus,
    robot_params::{RobotGateways, RobotPNL, RobotParams, RobotStrategyType},
    strategy::SimpleIncreaseDecreaseStrategy,
    Robot,
};
use crate::order_manager::OrderMsg;
use crate::platform::{self};
use crate::storage::robot::RobotDB;
use crate::storage::{RobotStore, Storage, StorageConnection};
use crate::{context_manager::ContextMsg, storage::SensorMsg};
use chrono::prelude::*;
use crossbeam::channel::{Receiver, Sender};
use std::str::FromStr;
use std::string::ToString;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

pub struct RobotEnvironment {
    pub robots: RwLock<Vec<Arc<Robot>>>,
}

impl<'a> RobotEnvironment {
    pub fn init() -> RobotEnvironment {
        RobotEnvironment {
            robots: RwLock::new(Vec::new()),
        }
    }

    // Initialize robots from configs files
    pub fn load_robot_environment(
        robot_configs: Vec<platform::config::Robot>,
        info_receiver: HashMap<String, Receiver<ContextMsg>>,
        order_sender: Sender<OrderMsg>,
        sensor_sender: Sender<SensorMsg>,
    ) -> Result<Self, &'static str> {
        let mut robots = Vec::new();
        for robot_config in robot_configs {
            robots.push(Arc::new(
                Robot::load(
                    &robot_config.config_file_path,
                    (*info_receiver.get(&robot_config.name).unwrap()).clone(),
                    order_sender.clone(),
                    sensor_sender.clone(),
                )
                .unwrap(),
            ));
        }
        Ok(RobotEnvironment {
            robots: RwLock::new(robots),
        })
    }

    // Inititialize robots from database
    // pub fn load_robots_from_db() -> RobotContext {
    //     let stored_robots = RobotStore::new_connection().load_all();
    //     match stored_robots {
    //         Ok(robots_db) => RobotContext {
    //             robots: RwLock::new(
    //                 robots_db
    //                     .iter()
    //                     .map(|robot_db| Robot {
    //                         params: RobotContext::from_robot_db(robot_db),
    //                         // ..Robot::generate("")
    //                     })
    //                     .collect(),
    //             ),
    //         },
    //         Err(_) => RobotContext {
    //             robots: RwLock::new(Vec::new()),
    //         },
    //     }
    // }

    pub fn graceful_shutdown(&self) {
        let robot_store = RobotStore::new_connection();
        let _ = self
            .robots
            .read()
            .unwrap()
            .iter()
            .map(|robot| robot_store.store(&RobotEnvironment::to_robot_db(robot)));
    }

    // Convert Robot
    fn from_robot_db(robot_db: &RobotDB) -> RobotParams {
        RobotParams {
            name: robot_db.name.clone(),
            strategy_type: RobotStrategyType::from_str(&robot_db.strategy).unwrap(),
            strategy: Box::new(SimpleIncreaseDecreaseStrategy::default()), // TODO
            gateway: RobotGateways::from_str(&robot_db.gateway).unwrap(),
            // TODO
            pnl: RobotPNL::default(),
        }
    }

    // Convert Robot
    fn to_robot_db(robot: &Robot) -> RobotDB {
        let robot_params = robot.robot_params.read().unwrap();
        RobotDB {
            id: 1,
            name: robot_params.name.clone(),
            gateway: robot_params.gateway.to_string(),
            strategy: robot_params.strategy_type.to_string(),
            instruments: vec![],
            timestamp: Utc::now().to_string(),
        }
    }

    fn _find(&self, robot_name: &str) -> Option<Arc<Robot>> {
        match self
            .robots
            .read()
            .unwrap()
            .iter()
            .position(|robot| robot.get_robot_name().unwrap() == robot_name)
        {
            Some(index) => match self.robots.read() {
                Ok(robots) => Some(robots[index].clone()),
                Err(_) => None,
            },
            None => None,
        }
    }

    // Find robot by name in Robot Environment
    fn find_robot(&self, robot_name: &str) -> Result<Arc<Robot>, &'static str> {
        match self._find(robot_name) {
            Some(robot) => Ok(robot.clone()),
            // None => Err(format!("Robot {} not found", robot_name).as_str()),
            None => Err("Robot not found"),
        }
    }

    pub fn start_robot(&self, robot_name: &str) -> Result<(), &'static str> {
        match self.find_robot(robot_name) {
            Ok(robot) => match Box::leak(Box::new(robot)).start() {
                Ok(_join_handle) => Ok(()),
                Err(e) => Err(e),
            },
            Err(_) => todo!(),
        }
    }

    pub fn stop_robot(&self, robot_name: &str) -> Result<(), &'static str> {
        self.find_robot(robot_name)?.stop()
    }

    pub fn status_robot(&self, robot_name: &str) -> Result<RobotStatus, &'static str> {
        match self.find_robot(robot_name)?.status() {
            Ok(status) => Ok(status.clone()),
            Err(_error_lock) => Err("Error lock"),
        }
    }

    pub fn info_robot(&self, robot_name: &str) -> Result<String, &'static str> {
        Ok(self.find_robot(robot_name)?.info()?)
    }

    pub fn set_config_robot(
        &self,
        robot_name: &str,
        config_file_path: &str,
    ) -> Result<(), &'static str> {
        self.find_robot(robot_name)?.set_config(config_file_path)
    }

    pub fn list_robots(&self) -> Result<Vec<String>, &'static str> {
        let mut info_list = Vec::new();
        for robot in Box::leak(Box::new(self.robots.read().unwrap())).iter() {
            info_list.push(robot.info()?);
        }
        Ok(info_list)
    }

    pub fn start_all_robots(&'static self) -> Result<(), &'static str> {
        match self.robots.read() {
            Ok(robots) => {
                for robot in Box::leak(Box::new(robots)).iter() {
                    robot.start()?;
                }
            }
            Err(_) => todo!(),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::robot::Robot;
    use crate::storage::robot::RobotDB;
    use crossbeam::channel::unbounded;
    use std::sync::Arc;

    impl<'a> RobotEnvironment {
        fn stub() -> Self {
            let robots = vec![
                Arc::new(Robot::generate("Robot1")),
                Arc::new(Robot::generate("Robot2")),
                Arc::new(Robot::generate("Robot3")),
            ];
            Self {
                robots: RwLock::new(robots),
            }
        }
    }

    #[test]
    fn load_robot_environment() {
        let robot_configs = vec![];
        let info_receiver = HashMap::new();
        let order_sender = unbounded().0;
        let sensor_sender = unbounded().0;

        assert!(RobotEnvironment::load_robot_environment(
            robot_configs,
            info_receiver,
            order_sender,
            sensor_sender,
        )
        .is_ok());
    }

    #[test]
    fn _find() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot1";
        assert!(robot_environment._find(robot_name).is_some());
    }

    #[test]
    fn find_robot() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot1";
        assert!(robot_environment.find_robot(robot_name).is_ok());
    }

    #[test]
    fn start_robot() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot1";
        assert!(robot_environment.start_robot(robot_name).is_ok());
    }

    #[test]
    #[ignore]
    fn stop_robot() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot1";
        robot_environment.start_robot(robot_name).unwrap();
        assert!(robot_environment.stop_robot(robot_name).is_ok());
    }

    #[test]
    fn stop_stopped_robot() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot1";
        assert!(robot_environment.stop_robot(robot_name).is_err());
    }

    #[test]
    fn status_robot() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot1";
        assert!(robot_environment.status_robot(robot_name).is_ok());
    }

    #[test]
    fn info_robot() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot1";
        assert!(robot_environment.info_robot(robot_name).is_ok());
    }

    #[test]
    fn set_config_robot() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot1";
        let config_file_path = "test_files/robot_config.toml";
        assert!(robot_environment
            .set_config_robot(robot_name, config_file_path)
            .is_ok());
    }

    #[test]
    fn list_robots() {
        let robot_environment = RobotEnvironment::stub();
        assert!(robot_environment.list_robots().is_ok());
    }

    #[test]
    fn start_all_robots() {
        let robot_environment = Box::leak(Box::new(RobotEnvironment::stub()));
        assert!(robot_environment.start_all_robots().is_ok());
    }

    #[test]
    fn test_from_robot_db() {
        let robot_db: RobotDB = RobotDB::default();
        let robot: Robot = Robot::default();
        let robot_from_robot_db = RobotEnvironment::from_robot_db(&robot_db);
        assert_eq!(robot_from_robot_db, *robot.robot_params.read().unwrap());
    }

    #[test]
    fn test_to_robot_db() {
        let robot_db: RobotDB = RobotDB::default();
        let robot: Robot = Robot::default();
        let robot_db_from_robot = RobotEnvironment::to_robot_db(&robot);

        assert_eq!(robot_db_from_robot, robot_db);
    }

    #[test]
    fn find_robot_found() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot1";
        let robot = robot_environment.find_robot(robot_name);
        assert!(robot.is_ok());
        assert_eq!(robot.unwrap().get_robot_name().unwrap(), robot_name);
    }

    #[test]
    fn find_robot_not_found() {
        let robot_environment = RobotEnvironment::stub();
        let robot_name = "Robot_not_found";
        let robot = robot_environment.find_robot(robot_name);
        assert!(robot.is_err());
    }
}
