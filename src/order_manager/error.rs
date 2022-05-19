#[derive(Debug)]
pub enum OrderManagerError {
    OrderManagerStartError(OrderManagerStartError),
    OrderManagerError(OrderManagerStopError),
}
#[derive(Debug)]
pub enum OrderManagerStartError {
    OrderManagerIsNotStartedError,
    OrderManagerIsAlreadyStartedError,
}

#[derive(Debug)]
pub enum OrderManagerStopError {
    OrderManagerIsNotStoppedError,
    OrderManagerIsAlreadyStoppedError,
}
