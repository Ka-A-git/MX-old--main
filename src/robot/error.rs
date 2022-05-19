#[derive(Debug)]
pub enum RobotError {
    RobotStartError(RobotStartError),
    RobotStopError(RobotStopError),
    RobotLockError(RobotLockError),
    RobotSetConfigError(RobotSetConfigError),
}

#[derive(Debug)]
pub enum RobotStartError {
    RobotIsNotStartedError,
    RobotIsAlreadyStarteddError,
}

#[derive(Debug)]
pub enum RobotStopError {
    RobotIsNotStoppedError,
    RobotIsAlreadyStoppeddError,
}

#[derive(Debug)]
pub enum RobotLockError {
    RobotIsNotLockedError,
    RobotIsAlreadyLockedError,
}

#[derive(Debug)]
pub enum RobotSetConfigError {
    RobotConfigNotFound,
}

#[derive(Debug)]
pub enum RobotEnvironmentError {
    RobotIsNotFoundError,
}
