use crate::gateway::GatewayEnvironment;
use crate::robot::RobotEnvironment;

pub trait EnvironmentAction<'a> {
    fn init() -> Self;

    fn graceful_shutdown(&self);
}

pub struct Environment {
    pub robot_environment: RobotEnvironment,

    pub gateway_environment: GatewayEnvironment,
}

impl Environment {
    pub fn init(
        robot_environment: RobotEnvironment,
        gateway_environment: GatewayEnvironment,
    ) -> Self {
        Environment {
            robot_environment,
            gateway_environment,
        }
    }

    fn graceful_shutdown(&self) {
        self.robot_environment.graceful_shutdown();
        self.gateway_environment.graceful_shutdown();
    }
}
