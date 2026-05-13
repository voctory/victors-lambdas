//! Structured logging utility.

mod config;
mod context;
mod entry;
mod format;
mod logger;
#[cfg(feature = "tracing")]
mod tracing_layer;
mod value;

pub use config::LoggerConfig;
pub use context::{LambdaContextFields, LambdaLogContext};
pub use entry::LogEntry;
pub use format::{JsonLogFormatter, LogFormatter, LogRedactor};
pub use logger::{LogLevel, Logger};
#[cfg(feature = "tracing")]
pub use tracing_layer::LoggerLayer;
pub use value::{LogFields, LogValue};

pub(crate) use value::normalize_key;
