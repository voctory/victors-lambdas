//! Structured logging utility.

mod config;
mod context;
mod entry;
mod format;
mod logger;
mod value;

pub use config::LoggerConfig;
pub use context::{LambdaContextFields, LambdaLogContext};
pub use entry::LogEntry;
pub use format::{JsonLogFormatter, LogFormatter, LogRedactor};
pub use logger::{LogLevel, Logger};
pub use value::{LogFields, LogValue};

pub(crate) use value::normalize_key;
