//! Structured logging utility.

mod config;
mod entry;
mod logger;
mod value;

pub use config::LoggerConfig;
pub use entry::LogEntry;
pub use logger::{LogLevel, Logger};
pub use value::{LogFields, LogValue};

pub(crate) use value::normalize_key;
