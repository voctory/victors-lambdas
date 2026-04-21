//! Umbrella crate for Powertools Lambda Rust utilities.

/// Batch processing utility.
#[cfg(feature = "batch")]
pub mod batch {
    pub use aws_lambda_powertools_batch::*;
}

/// Shared configuration and runtime helpers.
pub mod core {
    pub use aws_lambda_powertools_core::*;
}

/// Event handler utility.
#[cfg(feature = "event-handler")]
pub mod event_handler {
    pub use aws_lambda_powertools_event_handler::*;
}

/// Idempotency utility.
#[cfg(feature = "idempotency")]
pub mod idempotency {
    pub use aws_lambda_powertools_idempotency::*;
}

/// Structured logging utility.
#[cfg(feature = "logger")]
pub mod logger {
    pub use aws_lambda_powertools_logger::*;
}

/// Metrics utility.
#[cfg(feature = "metrics")]
pub mod metrics {
    pub use aws_lambda_powertools_metrics::*;
}

/// Parameter retrieval utility.
#[cfg(feature = "parameters")]
pub mod parameters {
    pub use aws_lambda_powertools_parameters::*;
}

/// Event parsing utility.
#[cfg(feature = "parser")]
pub mod parser {
    pub use aws_lambda_powertools_parser::*;
}

/// Common imports for Lambda handlers.
pub mod prelude {
    pub use aws_lambda_powertools_core::ServiceConfig;

    #[cfg(feature = "logger")]
    pub use aws_lambda_powertools_logger::{LogLevel, Logger, LoggerConfig};

    #[cfg(feature = "metrics")]
    pub use aws_lambda_powertools_metrics::{MetricUnit, Metrics, MetricsConfig};

    #[cfg(feature = "tracer")]
    pub use aws_lambda_powertools_tracer::{TraceContext, Tracer, TracerConfig};
}

/// Tracing utility.
#[cfg(feature = "tracer")]
pub mod tracer {
    pub use aws_lambda_powertools_tracer::*;
}

/// Validation utility.
#[cfg(feature = "validation")]
pub mod validation {
    pub use aws_lambda_powertools_validation::*;
}
