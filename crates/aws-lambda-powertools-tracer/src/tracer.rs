//! Tracer facade.

use crate::{TraceContext, TraceSegment, TracerConfig};

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

    /// Creates trace context for a segment name from an AWS X-Ray trace header.
    ///
    /// Supported header fields are `Root`, `Parent`, and `Sampled`.
    #[must_use]
    pub fn context_from_xray_header(&self, name: impl Into<String>, header: &str) -> TraceContext {
        TraceContext::from_xray_header(name, header)
    }

    /// Creates a trace segment record for a segment name.
    #[must_use]
    pub fn segment(&self, name: impl Into<String>) -> TraceSegment {
        self.segment_with_context(self.context(name))
    }

    /// Creates a trace segment record from an existing context.
    #[must_use]
    pub fn segment_with_context(&self, context: TraceContext) -> TraceSegment {
        TraceSegment::from_config(context, &self.config)
    }

    /// Returns tracer configuration.
    #[must_use]
    pub fn config(&self) -> &TracerConfig {
        &self.config
    }

    /// Returns the configured service name.
    #[must_use]
    pub fn service_name(&self) -> &str {
        self.config.service().service_name()
    }

    /// Returns whether trace data is being collected.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.config.enabled()
    }

    /// Returns whether handler responses are captured.
    #[must_use]
    pub fn captures_response(&self) -> bool {
        self.config.capture_response()
    }

    /// Returns whether handler errors are captured.
    #[must_use]
    pub fn captures_error(&self) -> bool {
        self.config.capture_error()
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{TraceValue, Tracer, TracerConfig};

    #[test]
    fn exposes_configured_service_and_flags() {
        let tracer = Tracer::with_config(
            TracerConfig::new("orders")
                .with_enabled(false)
                .with_capture_response(false)
                .with_capture_error(true),
        );

        assert_eq!(tracer.service_name(), "orders");
        assert!(!tracer.enabled());
        assert!(!tracer.captures_response());
        assert!(tracer.captures_error());
    }

    #[test]
    fn creates_segment_with_context_and_config_flags() {
        let tracer = Tracer::with_config(TracerConfig::new("orders").with_capture_response(false));
        let mut segment = tracer
            .segment("handler")
            .with_annotation("tenant", "north")
            .with_metadata("attempt", 2);

        segment.capture_response("ok").capture_error("failed");

        assert_eq!(segment.name(), "handler");
        assert_eq!(
            segment.annotations().get("tenant"),
            Some(&TraceValue::from("north"))
        );
        assert_eq!(segment.response(), None);
        assert_eq!(segment.error(), Some(&TraceValue::from("failed")));
    }

    #[test]
    fn creates_segment_from_xray_context() {
        let tracer = Tracer::with_config(TracerConfig::new("orders"));
        let context = tracer.context_from_xray_header(
            "handler",
            "Root=1-67891233-abcdef012345678912345678;Parent=53995c3f42cd8ad8;Sampled=0",
        );
        let segment = tracer.segment_with_context(context);

        assert_eq!(
            segment.context().trace_id(),
            Some("1-67891233-abcdef012345678912345678")
        );
        assert_eq!(segment.context().parent_id(), Some("53995c3f42cd8ad8"));
        assert_eq!(segment.context().sampled(), Some(false));
    }
}
