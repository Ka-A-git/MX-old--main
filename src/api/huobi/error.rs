use core::fmt;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum HuobiError {
    ApiError(String),
}

impl fmt::Display for HuobiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.clone() {
            HuobiError::ApiError(why) => write!(f, "ApiError: {}", why),
        }
    }
}

impl Error for HuobiError {
    fn description(&self) -> &str {
        "Huobi Error"
    }
}

#[derive(Debug)]
enum ApiError {}
