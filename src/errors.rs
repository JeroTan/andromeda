use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[error("{message}")]
pub struct Error {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl Error {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_argument", message)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self {
            code: "transport_error".into(),
            message: error.to_string(),
            retryable: true,
        }
    }
}
