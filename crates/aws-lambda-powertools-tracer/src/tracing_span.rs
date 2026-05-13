//! `tracing` span integration.

use tracing::{Level, Span, field};

use crate::{TraceFields, TraceSegment, TraceValue};

const TRACE_TARGET: &str = "aws_lambda_powertools_tracer";

impl TraceSegment {
    /// Converts this trace segment record into a `tracing` span.
    ///
    /// Segment context fields are attached using stable field names. Annotation,
    /// metadata, response, and error values are rendered as JSON strings so
    /// downstream subscribers can forward them to OpenTelemetry, X-Ray exporters,
    /// or structured logs without this crate selecting an exporter.
    #[must_use]
    pub fn to_span(&self) -> Span {
        if !self.enabled() {
            return Span::none();
        }

        let span = tracing::span!(
            target: TRACE_TARGET,
            Level::INFO,
            "powertools.trace",
            trace.name = self.name(),
            service.name = field::Empty,
            trace.id = field::Empty,
            trace.parent_id = field::Empty,
            trace.sampled = field::Empty,
            trace.annotations = field::Empty,
            trace.metadata = field::Empty,
            trace.response = field::Empty,
            trace.error = field::Empty,
        );

        if let Some(service_name) = self.service_name() {
            span.record("service.name", service_name);
        }
        if let Some(trace_id) = self.context().trace_id() {
            span.record("trace.id", trace_id);
        }
        if let Some(parent_id) = self.context().parent_id() {
            span.record("trace.parent_id", parent_id);
        }
        if let Some(sampled) = self.context().sampled() {
            span.record("trace.sampled", sampled);
        }
        if let Some(annotations) = fields_json(self.annotations()) {
            span.record("trace.annotations", annotations.as_str());
        }
        if let Some(metadata) = fields_json(self.metadata()) {
            span.record("trace.metadata", metadata.as_str());
        }
        if let Some(response) = self.response().map(TraceValue::to_json_string) {
            span.record("trace.response", response.as_str());
        }
        if let Some(error) = self.error().map(TraceValue::to_json_string) {
            span.record("trace.error", error.as_str());
        }

        span
    }
}

fn fields_json(fields: &TraceFields) -> Option<String> {
    (!fields.is_empty()).then(|| TraceValue::from(fields.clone()).to_json_string())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fmt,
        sync::{Arc, Mutex},
    };

    use tracing::{
        Subscriber,
        field::{Field, Visit},
        span::{Attributes, Id, Record},
        subscriber::with_default,
    };
    use tracing_subscriber::{
        Layer, Registry,
        layer::{Context, SubscriberExt},
    };

    use crate::{TraceContext, TraceValue, Tracer, TracerConfig};

    #[test]
    fn tracing_span_records_trace_fields() {
        let recorder = FieldRecorder::default();
        let subscriber = Registry::default().with(CaptureLayer {
            recorder: recorder.clone(),
        });

        with_default(subscriber, || {
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

            let span = segment.to_span();
            assert!(!span.is_disabled());
        });

        let fields = recorder.fields();
        assert_eq!(fields.get("trace.name"), Some(&"handler".to_owned()));
        assert_eq!(fields.get("service.name"), Some(&"orders".to_owned()));
        assert_eq!(
            fields.get("trace.id"),
            Some(&"1-67891233-abcdef012345678912345678".to_owned())
        );
        assert_eq!(
            fields.get("trace.parent_id"),
            Some(&"53995c3f42cd8ad8".to_owned())
        );
        assert_eq!(fields.get("trace.sampled"), Some(&"true".to_owned()));
        assert_eq!(
            fields.get("trace.annotations"),
            Some(&r#"{"tenant":"north"}"#.to_owned())
        );
        assert_eq!(
            fields.get("trace.metadata"),
            Some(&r#"{"payload":{"order_id":"order-1"}}"#.to_owned())
        );
        assert_eq!(fields.get("trace.response"), Some(&r#""ok""#.to_owned()));
        assert_eq!(fields.get("trace.error"), Some(&r#""failed""#.to_owned()));
    }

    #[derive(Clone, Default)]
    struct FieldRecorder {
        fields: Arc<Mutex<BTreeMap<String, String>>>,
    }

    impl FieldRecorder {
        fn fields(&self) -> BTreeMap<String, String> {
            self.fields
                .lock()
                .expect("field recorder should not be poisoned")
                .clone()
        }

        fn extend(&self, fields: BTreeMap<String, String>) {
            self.fields
                .lock()
                .expect("field recorder should not be poisoned")
                .extend(fields);
        }
    }

    struct CaptureLayer {
        recorder: FieldRecorder,
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber,
    {
        fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _context: Context<'_, S>) {
            let mut visitor = SpanFieldVisitor::default();
            attrs.record(&mut visitor);
            self.recorder.extend(visitor.fields);
        }

        fn on_record(&self, _id: &Id, values: &Record<'_>, _context: Context<'_, S>) {
            let mut visitor = SpanFieldVisitor::default();
            values.record(&mut visitor);
            self.recorder.extend(visitor.fields);
        }
    }

    #[derive(Default)]
    struct SpanFieldVisitor {
        fields: BTreeMap<String, String>,
    }

    impl SpanFieldVisitor {
        fn record_value(&mut self, field: &Field, value: impl Into<String>) {
            self.fields.insert(field.name().to_owned(), value.into());
        }
    }

    impl Visit for SpanFieldVisitor {
        fn record_bool(&mut self, field: &Field, value: bool) {
            self.record_value(field, value.to_string());
        }

        fn record_i64(&mut self, field: &Field, value: i64) {
            self.record_value(field, value.to_string());
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
            self.record_value(field, value.to_string());
        }

        fn record_i128(&mut self, field: &Field, value: i128) {
            self.record_value(field, value.to_string());
        }

        fn record_u128(&mut self, field: &Field, value: u128) {
            self.record_value(field, value.to_string());
        }

        fn record_f64(&mut self, field: &Field, value: f64) {
            self.record_value(field, value.to_string());
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.record_value(field, value);
        }

        fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
            self.record_value(field, format!("{value:?}"));
        }
    }
}
