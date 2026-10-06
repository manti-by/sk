//! A small error type: a message meant for humans, plus an optional cause.

use std::error::Error as StdError;
use std::fmt;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Error carrying a human-readable message and, optionally, its cause.
#[derive(Debug)]
pub struct Error {
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
}

impl Error {
    /// Creates an error without a cause.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// Creates an error from a message and its cause.
    pub fn with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn StdError + Send + Sync>>,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(source.into()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(source) = &self.source {
            write!(f, ": {source}")?;
        }
        Ok(())
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| &**source as &(dyn StdError + 'static))
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::with_source(error.to_string(), error)
    }
}

impl From<toml::de::Error> for Error {
    fn from(error: toml::de::Error) -> Self {
        Self::with_source(error.to_string(), error)
    }
}

impl From<toml::ser::Error> for Error {
    fn from(error: toml::ser::Error) -> Self {
        Self::with_source(error.to_string(), error)
    }
}

/// Returns early with `Error::new(..)`.
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return ::std::result::Result::Err($crate::error::Error::new(format!($($arg)*)))
    };
}

/// Returns early with `Error::new(..)` when `condition` is false.
#[macro_export]
macro_rules! ensure {
    ($condition:expr, $($arg:tt)*) => {
        if !$condition {
            return ::std::result::Result::Err($crate::error::Error::new(format!($($arg)*)));
        }
    };
}
