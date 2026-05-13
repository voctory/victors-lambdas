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

pub use config::TracerConfig;
pub use context::{TraceContext, XRAY_TRACE_HEADER_NAME};
pub use segment::TraceSegment;
pub use tracer::Tracer;
pub use value::{TraceFields, TraceValue};
#[cfg(feature = "xray")]
pub use xray::{XrayDocumentError, XrayDocumentResult};

pub(crate) use value::{normalize_key, write_json_string};
