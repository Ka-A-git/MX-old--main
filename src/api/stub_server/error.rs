use core::fmt;
use std::error::Error;

pub type APIResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone)]
pub enum StubServerError {
    ApiError(String),
}

impl fmt::Display for StubServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.clone() {
            StubServerError::ApiError(why) => write!(f, "ApiError: {}", why),
        }
    }
}

impl Error for StubServerError {
    fn description(&self) -> &str {
        "Stub Server Error"
    }
}

#[derive(Debug)]
enum ApiError {}
