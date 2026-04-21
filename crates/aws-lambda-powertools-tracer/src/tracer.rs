//! Tracer facade.

use crate::{TraceContext, TracerConfig};

/// Tracer facade for Lambda handlers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tracer {
    config: TracerConfig,
}

impl Tracer {
    /// Creates a tracer from environment configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(TracerConfig::from_env())
    }

    /// Creates a tracer with explicit configuration.
    #[must_use]
    pub fn with_config(config: TracerConfig) -> Self {
        Self { config }
    }

    /// Creates trace context for a segment name.
    #[must_use]
    pub fn context(&self, name: impl Into<String>) -> TraceContext {
        TraceContext::new(name)
    }

    /// Returns tracer configuration.
    #[must_use]
    pub fn config(&self) -> &TracerConfig {
        &self.config
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new()
    }
}
