//! Tracing utility.

mod config;
mod context;
mod segment;
mod tracer;
#[cfg(feature = "tracing")]
mod tracing_span;
mod value;
#[cfg(feature = "xray")]
mod xray;
#[cfg(feature = "xray-daemon")]
mod xray_daemon;

pub use config::TracerConfig;
pub use context::{TraceContext, XRAY_TRACE_HEADER_NAME};
pub use segment::TraceSegment;
pub use tracer::Tracer;
pub use value::{TraceFields, TraceValue};
#[cfg(feature = "xray")]
pub use xray::{XrayDocumentError, XrayDocumentResult};
#[cfg(feature = "xray-daemon")]
pub use xray_daemon::{XrayDaemonClient, XrayDaemonConfig, XrayDaemonError};

pub(crate) use value::{normalize_key, write_json_string};
