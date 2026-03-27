//! Error types for torchforge-data
//!
//! This module defines the error types used throughout the library.

use thiserror::Error;

/// Result type alias for the library
pub type Result<T> = std::result::Result<T, DataError>;

/// Main error type for torchforge-data operations
#[derive(Error, Debug)]
pub enum DataError {
    /// I/O related errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Memory mapping related errors
    #[error("Memory mapping error: {0}")]
    Mmap(String),

    /// Data format errors
    #[error("Data format error: {0}")]
    Format(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Buffer capacity errors
    #[error("Buffer capacity error: {0}")]
    Capacity(String),
}
