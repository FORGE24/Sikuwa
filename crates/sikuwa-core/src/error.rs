//! Unified error types for Sikuwa 2.0.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SikuwaError>;

#[derive(Debug, Error)]
pub enum SikuwaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("PythonIR error: {0}")]
    Pir(String),

    #[error("PyStat error: {0}")]
    Pystat(String),

    #[error("backend error: {0}")]
    Backend(String),

    #[error("{0}")]
    Other(String),
}

impl SikuwaError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn pir(msg: impl Into<String>) -> Self {
        Self::Pir(msg.into())
    }
}
