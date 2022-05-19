#[derive(Debug)]
enum PlatformError {
    PlatformStartError(PlatformStartError),
    PlatformStopError(PlatformStopError),
    SetConfigError(SetConfigError),
}

#[derive(Debug)]
enum PlatformStartError {}

#[derive(Debug)]
enum PlatformStopError {}

#[derive(Debug)]
enum SetConfigError {
    PlatformConfigNotFound,
}
