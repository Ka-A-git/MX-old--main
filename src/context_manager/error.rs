#[derive(Debug)]
enum ContextManagerError {
    ContextManagerStartError(ContextManagerStartError),
    ContextManagerStopError(ContextManagerStopError),
    ContextManagerPublishInfoError,
    ContextManagerUpdateInfoError,
}
#[derive(Debug)]
enum ContextManagerStartError {}
#[derive(Debug)]
enum ContextManagerStopError {}
