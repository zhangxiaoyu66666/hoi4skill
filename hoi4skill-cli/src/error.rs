//! CLI-level error types.
//!
//! Most legacy command functions still return plain strings while the monolith
//! is being split. The binary entrypoint now has a typed boundary so future
//! modules can migrate toward structured variants without changing `main`.

use std::error::Error;
use std::fmt;

pub type CliResult<T> = Result<T, CliError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    Usage(String),
    Message(String),
}

impl CliError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }

    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Message(message) => f.write_str(message),
        }
    }
}

impl Error for CliError {}

impl From<String> for CliError {
    fn from(value: String) -> Self {
        Self::message(value)
    }
}

impl From<&str> for CliError {
    fn from(value: &str) -> Self {
        Self::message(value)
    }
}
