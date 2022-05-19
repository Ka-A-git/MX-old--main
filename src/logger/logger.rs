use tracing_appender;
use tracing_subscriber;

const EVENTS_LOG_FILE: &str = "events.log";
const EVENTS_LOG_DIR: &str = "./data";

pub struct Logger;

impl Logger {
    pub fn init() {
        println!(
            "Server writes all events to the log file: {}/{}",
            EVENTS_LOG_DIR, EVENTS_LOG_FILE
        );
        let file_appender = tracing_appender::rolling::never(EVENTS_LOG_DIR, EVENTS_LOG_FILE);
        let (_non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::fmt()
            // .with_writer(non_blocking)
            // .with_max_level(tracing::Level::DEBUG)
            .with_max_level(tracing::Level::INFO)
            .init();
    }
}
