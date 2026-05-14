//! OpenTelemetry export helpers.

use opentelemetry::{
    KeyValue,
    trace::{Span, SpanBuilder, SpanKind},
};

use crate::{TraceFields, TraceSegment, TraceValue};

impl TraceSegment {
    /// Converts this trace segment record into OpenTelemetry span attributes.
    ///
    /// Annotation, metadata, response, and error values are rendered as JSON
    /// strings so callers can use any OpenTelemetry SDK or exporter without
    /// this crate selecting one.
    #[must_use]
    pub fn to_otel_attributes(&self) -> Vec<KeyValue> {
        if !self.enabled() {
            return Vec::new();
        }

        let mut attributes = vec![KeyValue::new("trace.name", self.name().to_owned())];
        if let Some(service_name) = self.service_name() {
            attributes.push(KeyValue::new("service.name", service_name.to_owned()));
        }
        if let Some(trace_id) = self.context().trace_id() {
            attributes.push(KeyValue::new("trace.id", trace_id.to_owned()));
        }
        if let Some(parent_id) = self.context().parent_id() {
            attributes.push(KeyValue::new("trace.parent_id", parent_id.to_owned()));
        }
        if let Some(sampled) = self.context().sampled() {
            attributes.push(KeyValue::new("trace.sampled", sampled));
        }
        if let Some(annotations) = fields_json(self.annotations()) {
            attributes.push(KeyValue::new("trace.annotations", annotations));
        }
        if let Some(metadata) = fields_json(self.metadata()) {
            attributes.push(KeyValue::new("trace.metadata", metadata));
        }
        if let Some(response) = self.response().map(TraceValue::to_json_string) {
            attributes.push(KeyValue::new("trace.response", response));
        }
        if let Some(error) = self.error().map(TraceValue::to_json_string) {
            attributes.push(KeyValue::new("trace.error", error));
        }

        attributes
    }

    /// Converts this trace segment record into an OpenTelemetry span builder.
    ///
    /// The builder uses the segment name as the span name and attaches the same
    /// attributes returned by [`TraceSegment::to_otel_attributes`].
    #[must_use]
    pub fn to_otel_span_builder(&self) -> SpanBuilder {
        SpanBuilder::from_name(self.name().to_owned())
            .with_kind(SpanKind::Internal)
            .with_attributes(self.to_otel_attributes())
    }

    /// Records this trace segment record onto an existing OpenTelemetry span.
    pub fn record_otel_attributes<S>(&self, span: &mut S)
    where
        S: Span,
    {
        span.set_attributes(self.to_otel_attributes());
    }
}

fn fields_json(fields: &TraceFields) -> Option<String> {
    (!fields.is_empty()).then(|| TraceValue::from(fields.clone()).to_json_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{TraceContext, TraceValue, Tracer, TracerConfig};

    #[test]
    fn otel_attributes_record_trace_fields() {
        let tracer = Tracer::with_config(TracerConfig::new("orders"));
        let segment = tracer
            .segment_with_context(
                TraceContext::new("handler")
                    .with_trace_id("1-67891233-abcdef012345678912345678")
                    .with_parent_id("53995c3f42cd8ad8")
                    .with_sampled(true),
            )
            .with_annotation("tenant", "north")
            .with_metadata("payload", TraceValue::object([("order_id", "order-1")]))
            .with_response("ok")
            .with_error("failed");

        let attributes = attributes_by_key(segment.to_otel_attributes());

        assert_eq!(attributes.get("trace.name"), Some(&"handler".to_owned()));
        assert_eq!(attributes.get("service.name"), Some(&"orders".to_owned()));
        assert_eq!(
            attributes.get("trace.id"),
            Some(&"1-67891233-abcdef012345678912345678".to_owned())
        );
        assert_eq!(
            attributes.get("trace.parent_id"),
            Some(&"53995c3f42cd8ad8".to_owned())
        );
        assert_eq!(attributes.get("trace.sampled"), Some(&"true".to_owned()));
        assert_eq!(
            attributes.get("trace.annotations"),
            Some(&r#"{"tenant":"north"}"#.to_owned())
        );
        assert_eq!(
            attributes.get("trace.metadata"),
            Some(&r#"{"payload":{"order_id":"order-1"}}"#.to_owned())
        );
        assert_eq!(
            attributes.get("trace.response"),
            Some(&r#""ok""#.to_owned())
        );
        assert_eq!(
            attributes.get("trace.error"),
            Some(&r#""failed""#.to_owned())
        );
    }

    #[test]
    fn otel_span_builder_uses_segment_name_and_attributes() {
        let tracer = Tracer::with_config(TracerConfig::new("orders"));
        let builder = tracer.segment("handler").to_otel_span_builder();

        assert_eq!(builder.name.as_ref(), "handler");
        assert_eq!(
            builder
                .attributes
                .expect("builder has attributes")
                .iter()
                .map(|attribute| attribute.key.as_str())
                .collect::<Vec<_>>(),
            ["trace.name", "service.name"]
        );
    }

    #[test]
    fn disabled_segment_exports_no_otel_attributes() {
        let segment =
            Tracer::with_config(TracerConfig::new("orders").with_enabled(false)).segment("handler");

        assert!(segment.to_otel_attributes().is_empty());
    }

    fn attributes_by_key(attributes: Vec<opentelemetry::KeyValue>) -> BTreeMap<String, String> {
        attributes
            .into_iter()
            .map(|attribute| {
                (
                    attribute.key.as_str().to_owned(),
                    value_to_string(&attribute.value),
                )
            })
            .collect()
    }

    fn value_to_string(value: &opentelemetry::Value) -> String {
        value.to_string()
    }
}
