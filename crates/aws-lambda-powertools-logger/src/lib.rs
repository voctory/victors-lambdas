//! Structured logging utility.

mod config;
mod logger;

pub use config::LoggerConfig;
pub use logger::{LogLevel, Logger};
