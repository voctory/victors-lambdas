//! Trace segment records.

use crate::{TraceContext, TraceFields, TraceValue, TracerConfig, normalize_key};

/// A trace segment or subsegment being prepared for export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceSegment {
    context: TraceContext,
    annotations: TraceFields,
    metadata: TraceFields,
    response: Option<TraceValue>,
    error: Option<TraceValue>,
    enabled: bool,
    capture_response: bool,
    capture_error: bool,
}

impl TraceSegment {
    /// Creates a trace segment with default capture flags enabled.
    #[must_use]
    pub fn new(context: TraceContext) -> Self {
        Self {
            context,
            annotations: TraceFields::new(),
            metadata: TraceFields::new(),
            response: None,
            error: None,
            enabled: true,
            capture_response: true,
            capture_error: true,
        }
    }

    pub(crate) fn from_config(context: TraceContext, config: &TracerConfig) -> Self {
        Self {
            context,
            annotations: TraceFields::new(),
            metadata: TraceFields::new(),
            response: None,
            error: None,
            enabled: config.enabled(),
            capture_response: config.capture_response(),
            capture_error: config.capture_error(),
        }
    }

    /// Returns a copy of this segment with tracing enabled or disabled.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        if !enabled {
            self.clear();
        }
        self
    }

    /// Returns a copy of this segment with response capture enabled or disabled.
    #[must_use]
    pub fn with_capture_response(mut self, capture_response: bool) -> Self {
        self.capture_response = capture_response;
        if !capture_response {
            self.response = None;
        }
        self
    }

    /// Returns a copy of this segment with error capture enabled or disabled.
    #[must_use]
    pub fn with_capture_error(mut self, capture_error: bool) -> Self {
        self.capture_error = capture_error;
        if !capture_error {
            self.error = None;
        }
        self
    }

    /// Returns a copy of this segment with an annotation.
    ///
    /// Annotations are intended for small indexed values. Blank annotation names
    /// are ignored.
    #[must_use]
    pub fn with_annotation(mut self, key: impl Into<String>, value: impl Into<TraceValue>) -> Self {
        self.add_annotation(key, value);
        self
    }

    /// Returns a copy of this segment with metadata.
    ///
    /// Metadata is intended for larger diagnostic values. Blank metadata names
    /// are ignored.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<TraceValue>) -> Self {
        self.add_metadata(key, value);
        self
    }

    /// Returns a copy of this segment with a captured response when enabled.
    #[must_use]
    pub fn with_response(mut self, response: impl Into<TraceValue>) -> Self {
        self.capture_response(response);
        self
    }

    /// Returns a copy of this segment with a captured error when enabled.
    #[must_use]
    pub fn with_error(mut self, error: impl Into<TraceValue>) -> Self {
        self.capture_error(error);
        self
    }

    /// Adds or replaces an annotation.
    ///
    /// Blank annotation names are ignored. When tracing is disabled, this is a
    /// no-op.
    pub fn add_annotation(
        &mut self,
        key: impl Into<String>,
        value: impl Into<TraceValue>,
    ) -> &mut Self {
        if self.enabled {
            if let Some(key) = normalize_key(key) {
                self.annotations.insert(key, value.into());
            }
        }
        self
    }

    /// Adds or replaces metadata.
    ///
    /// Blank metadata names are ignored. When tracing is disabled, this is a
    /// no-op.
    pub fn add_metadata(
        &mut self,
        key: impl Into<String>,
        value: impl Into<TraceValue>,
    ) -> &mut Self {
        if self.enabled {
            if let Some(key) = normalize_key(key) {
                self.metadata.insert(key, value.into());
            }
        }
        self
    }

    /// Captures a handler response when tracing and response capture are enabled.
    pub fn capture_response(&mut self, response: impl Into<TraceValue>) -> &mut Self {
        if self.enabled && self.capture_response {
            self.response = Some(response.into());
        }
        self
    }

    /// Captures a handler error when tracing and error capture are enabled.
    pub fn capture_error(&mut self, error: impl Into<TraceValue>) -> &mut Self {
        if self.enabled && self.capture_error {
            self.error = Some(error.into());
        }
        self
    }

    /// Removes all annotations, metadata, captured responses, and captured errors.
    pub fn clear(&mut self) {
        self.annotations.clear();
        self.metadata.clear();
        self.response = None;
        self.error = None;
    }

    /// Returns the trace context.
    #[must_use]
    pub const fn context(&self) -> &TraceContext {
        &self.context
    }

    /// Returns the segment name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.context.name()
    }

    /// Returns configured annotations.
    #[must_use]
    pub const fn annotations(&self) -> &TraceFields {
        &self.annotations
    }

    /// Returns configured metadata.
    #[must_use]
    pub const fn metadata(&self) -> &TraceFields {
        &self.metadata
    }

    /// Returns the captured response, if any.
    #[must_use]
    pub const fn response(&self) -> Option<&TraceValue> {
        self.response.as_ref()
    }

    /// Returns the captured error, if any.
    #[must_use]
    pub const fn error(&self) -> Option<&TraceValue> {
        self.error.as_ref()
    }

    /// Returns whether trace data is being collected.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns whether responses are captured when tracing is enabled.
    #[must_use]
    pub const fn captures_response(&self) -> bool {
        self.capture_response
    }

    /// Returns whether errors are captured when tracing is enabled.
    #[must_use]
    pub const fn captures_error(&self) -> bool {
        self.capture_error
    }
}

#[cfg(test)]
mod tests {
    use crate::{TraceContext, TraceSegment, TraceValue, TracerConfig};

    #[test]
    fn stores_annotations_metadata_and_capture_values() {
        let mut segment = TraceSegment::new(TraceContext::new("handler"))
            .with_annotation("tenant", "north")
            .with_metadata("request", TraceValue::object([("id", "evt-1")]))
            .with_response("ok");

        segment.capture_error("failed");

        assert_eq!(segment.name(), "handler");
        assert_eq!(
            segment.annotations().get("tenant"),
            Some(&TraceValue::from("north"))
        );
        assert_eq!(
            segment
                .metadata()
                .get("request")
                .map(TraceValue::to_json_string),
            Some("{\"id\":\"evt-1\"}".to_owned())
        );
        assert_eq!(segment.response(), Some(&TraceValue::from("ok")));
        assert_eq!(segment.error(), Some(&TraceValue::from("failed")));
    }

    #[test]
    fn ignores_blank_keys_and_keeps_fields_ordered() {
        let segment = TraceSegment::new(TraceContext::new("handler"))
            .with_annotation("zeta", true)
            .with_annotation("  ", "ignored")
            .with_annotation("alpha", 1);

        assert_eq!(
            segment
                .annotations()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
    }

    #[test]
    fn config_flags_control_capture_behavior() {
        let config = TracerConfig::new("orders")
            .with_capture_response(false)
            .with_capture_error(false);
        let mut segment = TraceSegment::from_config(TraceContext::new("handler"), &config);

        segment.capture_response("ok").capture_error("failed");

        assert_eq!(segment.response(), None);
        assert_eq!(segment.error(), None);
        assert!(!segment.captures_response());
        assert!(!segment.captures_error());
    }

    #[test]
    fn disabled_segment_does_not_collect_values() {
        let mut segment = TraceSegment::new(TraceContext::new("handler")).with_enabled(false);

        segment
            .add_annotation("tenant", "north")
            .add_metadata("payload", "value")
            .capture_response("ok")
            .capture_error("failed");

        assert!(segment.annotations().is_empty());
        assert!(segment.metadata().is_empty());
        assert_eq!(segment.response(), None);
        assert_eq!(segment.error(), None);
    }

    #[test]
    fn disabling_segment_clears_previously_collected_values() {
        let segment = TraceSegment::new(TraceContext::new("handler"))
            .with_annotation("tenant", "north")
            .with_metadata("payload", "value")
            .with_response("ok")
            .with_error("failed")
            .with_enabled(false);

        assert!(segment.annotations().is_empty());
        assert!(segment.metadata().is_empty());
        assert_eq!(segment.response(), None);
        assert_eq!(segment.error(), None);
        assert!(!segment.enabled());
    }

    #[test]
    fn clear_removes_collected_values() {
        let mut segment = TraceSegment::new(TraceContext::new("handler"))
            .with_annotation("tenant", "north")
            .with_metadata("payload", "value")
            .with_response("ok")
            .with_error("failed");

        segment.clear();

        assert!(segment.annotations().is_empty());
        assert!(segment.metadata().is_empty());
        assert_eq!(segment.response(), None);
        assert_eq!(segment.error(), None);
    }
}
