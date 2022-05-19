use super::{Gateway, GatewayStatus};
use crate::context_manager::GatewayMsg;
use crate::order_manager::{ActiveOrderMsg, OrderMsg};
use crate::platform::{self, EnvironmentAction};
use crossbeam::channel::{Receiver, Sender};
use std::{collections::HashMap, sync::RwLock};

pub struct GatewayEnvironment {
    pub gateways: &'static RwLock<Vec<Gateway>>, // remove pub
    robot_configs: Vec<platform::config::Robot>,
}

impl GatewayEnvironment {
    pub fn load_gateway_environment(
        robot_configs: Vec<platform::config::Robot>,
        gateway_configs: Vec<platform::config::Gateway>,
        info_sender: Sender<GatewayMsg>,
        order_receiver: HashMap<String, Receiver<OrderMsg>>,
        active_order_sender: Sender<ActiveOrderMsg>,
    ) -> Result<Self, &'static str> {
        let mut gateways = Vec::new();
        for gateway_config in gateway_configs {
            gateways.push(
                Gateway::load(
                    &gateway_config.config_file_path,
                    info_sender.clone(),
                    (*order_receiver.get(&gateway_config.name).unwrap()).clone(),
                    active_order_sender.clone(),
                )
                .unwrap(),
            )
        }
        Ok(GatewayEnvironment {
            robot_configs,
            gateways: Box::leak(Box::new(RwLock::new(gateways))),
        })
    }

    pub fn get_dependent_robots(&self, gateway_name: &str) -> Vec<String> {
        match Gateway::dependent_robots(self.robot_configs.clone()).get(gateway_name) {
            Some(robots) => robots.clone(),
            None => vec![],
        }
    }

    // Finds Gateway by name and starts it
    pub fn start_gateway(&self, gateway_name: &str) -> Result<(), &'static str> {
        match self.find_gateway(gateway_name)?.start() {
            Ok(_join_handle) => Ok(()),
            Err(e) => Err(e),
        }
    }

    // Finds Gateway by name and stops it
    pub fn stop_gateway(&self, gateway_name: &str) -> Result<(), &'static str> {
        self.find_gateway(gateway_name)?.stop()
    }

    // Finds Gateway by name and gets its status
    pub fn status_gateway(&self, gateway_name: &str) -> Result<GatewayStatus, &'static str> {
        match self.find_gateway(gateway_name)?.status() {
            Ok(status) => {
                // make a string
                Ok(status)
            }
            Err(_e) => Err("Gateway status error"),
        }
    }

    // Finds Gateway by name and get info about it
    pub fn info_gateway(&self, gateway_name: &str) -> Result<String, &'static str> {
        Ok(self.find_gateway(gateway_name)?.info()?)
    }

    pub fn set_config_gateway(
        &self,
        gateway_name: &str,
        config_file_path: &str,
    ) -> Result<(), &'static str> {
        self.find_gateway(gateway_name)?
            .set_config(config_file_path)
    }

    pub fn up_gateways(&self) -> Result<(), &'static str> {
        for gateway in Box::leak(Box::new(self.gateways.read().unwrap())).iter() {
            match gateway.start() {
                Ok(_) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn list_gateways(&self) -> Result<Vec<String>, &'static str> {
        let mut info_list = Vec::new();

        for gateway in Box::leak(Box::new(self.gateways.read().unwrap())).iter() {
            info_list.push(gateway.info()?);
        }

        Ok(info_list)
    }

    fn find_gateway(&self, gateway_name: &str) -> Result<&'static Gateway, &'static str> {
        let index = self.get_index_by_name(gateway_name)?;
        let gateways = Box::leak(Box::new(self.gateways.read().unwrap()));
        Ok(&gateways[index])
    }

    fn get_index_by_name(&self, gateway_name: &str) -> Result<usize, &'static str> {
        match self
            .gateways
            .read()
            .unwrap()
            .iter()
            .position(|g| g.get_gateway_name().unwrap() == gateway_name)
        {
            Some(index) => Ok(index),
            None => Err(""),
        }
    }

    pub fn start_all_gateways(&self) -> Result<(), &'static str> {
        for gateway in Box::leak(Box::new(self.gateways.read().unwrap())).iter() {
            gateway.start()?;
        }
        Ok(())
    }

    pub fn get_gateways(&self) -> Result<Vec<Gateway>, &'static str> {
        match self.gateways.read() {
            Ok(gateways) => Ok(gateways.clone()),
            Err(_e) => Err("err"),
        }
    }
}

impl<'a> EnvironmentAction<'a> for GatewayEnvironment {
    fn init() -> Self {
        let robot_configs = vec![];
        GatewayEnvironment {
            robot_configs,
            gateways: Box::leak(Box::new(RwLock::new(vec![]))),
            // gateways: RwLock::new(vec![]),
        }
    }

    fn graceful_shutdown(&self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use super::GatewayEnvironment;
    use crate::config::get_config;
    use crate::gateway::GatewayStatus;
    use crate::platform::{PlatformConfig, PlatformUtils};
    use crossbeam::channel::unbounded;
    use std::collections::HashMap;

    impl GatewayEnvironment {
        fn stub() -> Self {
            let test_platform_config_file_path = "test_files/platform_config.toml";

            let platform_config: PlatformConfig =
                get_config(test_platform_config_file_path).unwrap();

            let info_sender = unbounded().0;
            let order_receiver = PlatformUtils::get_gateway_channels(&platform_config).1;
            let active_order_sender = unbounded().0;

            GatewayEnvironment::load_gateway_environment(
                platform_config.robots,
                platform_config.gateways,
                info_sender,
                order_receiver,
                active_order_sender,
            )
            .unwrap()
        }
    }

    #[test]
    fn load_gateway_environment() {
        let robot_configs = Vec::new();
        let gateway_configs = Vec::new();
        let info_sender = unbounded().0;
        let order_receiver = HashMap::new();
        let active_order_sender = unbounded().0;

        assert!(GatewayEnvironment::load_gateway_environment(
            robot_configs,
            gateway_configs,
            info_sender,
            order_receiver,
            active_order_sender,
        )
        .is_ok());
    }

    #[test]
    fn get_dependent_robots() {
        let gateway_environment = GatewayEnvironment::stub();
        let gateway_name = "GatewayStub";
        gateway_environment.get_dependent_robots(gateway_name);
    }

    #[test]
    fn start_gateway() {
        let gateway_environment = GatewayEnvironment::stub();
        let gateway_name = "GatewayStub";
        assert!(gateway_environment.start_gateway(gateway_name).is_ok());
    }

    #[test]
    fn stop_gateway() {
        let gateway_environment = GatewayEnvironment::stub();
        let gateway_name = "GatewayStub";
        assert!(gateway_environment.stop_gateway(gateway_name).is_ok());
    }

    #[test]
    fn status_gateway() {
        let gateway_environment = GatewayEnvironment::stub();
        let gateway_name = "GatewayStub";
        assert!(gateway_environment.status_gateway(gateway_name).is_ok());
    }

    #[test]
    fn info_gateway() {
        let gateway_environment = GatewayEnvironment::stub();
        let gateway_name = "GatewayStub";
        assert!(gateway_environment.info_gateway(gateway_name).is_ok());
    }

    #[test]
    fn set_config_gateway() {
        let gateway_environment = GatewayEnvironment::stub();
        let gateway_name = "GatewayStub";
        let config_file_path = "";
        assert!(gateway_environment
            .set_config_gateway(gateway_name, config_file_path)
            .is_ok());
    }

    #[tokio::test]
    async fn up_gateways() {
        let gateway_environment = GatewayEnvironment::stub();
        assert!(gateway_environment.up_gateways().is_ok());
    }

    #[test]
    fn list_gateways() {
        let gateway_environment = GatewayEnvironment::stub();
        assert!(gateway_environment.list_gateways().is_ok());
    }

    #[test]
    fn find_gateway() {
        let gateway_environment = GatewayEnvironment::stub();
        let gateway_name = "GatewayStub";
        assert!(gateway_environment.find_gateway(gateway_name).is_ok());
    }

    #[test]
    fn get_index_by_name() {
        let gateway_environment = GatewayEnvironment::stub();
        let gateway_name = "GatewayStub";
        assert!(gateway_environment.get_index_by_name(gateway_name).is_ok());
    }

    #[tokio::test]
    async fn start_all_gateways() {
        let gateway_environment = GatewayEnvironment::stub();
        assert!(gateway_environment.start_all_gateways().is_ok());
    }

    #[tokio::test]
    async fn clone_env() {
        let gateway_environment = GatewayEnvironment::stub();

        let gateways1 = Box::leak(Box::new(gateway_environment.get_gateways().unwrap()));
        let g1 = gateways1.first().unwrap();
        assert_eq!(g1.status().unwrap(), GatewayStatus::Stopped);
        g1.start().unwrap();
        assert_eq!(g1.status().unwrap(), GatewayStatus::Active);

        let gateways2 = Box::leak(Box::new(gateway_environment.get_gateways().unwrap()));
        let g2 = gateways2.first().unwrap();
        assert_eq!(g2.status().unwrap(), GatewayStatus::Active);
    }
}
