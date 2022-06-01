#[derive(Debug)]
pub enum GatewayError {
    GatewayStartError(GatewayStartError),
    GatewayStopError(GatewayStopError), 
    FetchBalanceError(FetchBalanceError),
    BuyInstrumentError(BuyInstrumentError),
    SellInstrumentError(SellInstrumentError),
    SetConfigError(SetConfigError),
}
 
#[derive(Debug)]
pub enum GatewayStartError {
    GatewayIsNotStartedError,
    GatewayIsAlreadyStartedError,
}

#[derive(Debug)]
pub enum GatewayStopError {
    GatewayIsNotStoppedError,
    GatewayIsAlreadyStoppedError,
}

#[derive(Debug)]
pub enum FetchBalanceError {
    FetchBalanceBinanceError,
    FetchBalanceHuobiError,
}
#[derive(Debug)]
pub enum BuyInstrumentError {
    BuyInstrumentBinanceError,
    BuyInstrumentHuobiError,
}

#[derive(Debug)]
pub enum SellInstrumentError {
    SellInstrumentBinanceError,
    SellInstrumentHuobiError,
}

#[derive(Debug)]
pub enum SetConfigError {
    GatewayConfigNotFound,
}
