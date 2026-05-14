//! AWS X-Ray segment document rendering.

use std::fmt;

use crate::{TraceFields, TraceSegment, TraceValue, write_json_string};

/// Result returned by X-Ray document rendering.
pub type XrayDocumentResult<T> = Result<T, XrayDocumentError>;

/// Error returned when a trace segment cannot be rendered as an X-Ray document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum XrayDocumentError {
    /// Trace collection is disabled for the segment.
    Disabled,
    /// The segment context does not include an X-Ray trace identifier.
    MissingTraceId,
    /// The segment context does not include a parent segment identifier.
    MissingParentId,
    /// The supplied subsegment identifier is empty.
    EmptyId,
    /// The supplied start or end timestamp is not finite, or the end precedes the start.
    InvalidTimestamps,
}

impl fmt::Display for XrayDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => formatter.write_str("trace segment is disabled"),
            Self::MissingTraceId => formatter.write_str("trace segment is missing trace_id"),
            Self::MissingParentId => formatter.write_str("trace segment is missing parent_id"),
            Self::EmptyId => formatter.write_str("X-Ray subsegment id cannot be empty"),
            Self::InvalidTimestamps => {
                formatter.write_str("X-Ray subsegment timestamps are invalid")
            }
        }
    }
}

impl std::error::Error for XrayDocumentError {}

impl TraceSegment {
    /// Renders this segment as an X-Ray subsegment document.
    ///
    /// The caller supplies the subsegment `id` and epoch-second start/end
    /// timestamps so this crate does not need a random ID generator or hidden
    /// clock. Captured metadata, responses, and errors are rendered in the
    /// configured service namespace, or `default` when no service is attached.
    ///
    /// # Errors
    ///
    /// Returns [`XrayDocumentError`] when the segment is disabled, the context
    /// is missing required X-Ray identifiers, the supplied id is empty, or the
    /// supplied timestamps are invalid.
    pub fn to_xray_subsegment_document(
        &self,
        id: impl Into<String>,
        start_time: f64,
        end_time: f64,
    ) -> XrayDocumentResult<String> {
        if !self.enabled() {
            return Err(XrayDocumentError::Disabled);
        }
        if !start_time.is_finite() || !end_time.is_finite() || end_time < start_time {
            return Err(XrayDocumentError::InvalidTimestamps);
        }

        let trace_id = self
            .context()
            .trace_id()
            .ok_or(XrayDocumentError::MissingTraceId)?;
        let parent_id = self
            .context()
            .parent_id()
            .ok_or(XrayDocumentError::MissingParentId)?;
        let id = id.into();
        let id = id.trim();
        if id.is_empty() {
            return Err(XrayDocumentError::EmptyId);
        }

        let mut output = String::new();
        output.push('{');
        let mut fields = 0;

        write_string_field(&mut output, &mut fields, "name", self.name());
        write_string_field(&mut output, &mut fields, "id", id);
        write_string_field(&mut output, &mut fields, "trace_id", trace_id);
        write_string_field(&mut output, &mut fields, "parent_id", parent_id);
        write_number_field(&mut output, &mut fields, "start_time", start_time);
        write_number_field(&mut output, &mut fields, "end_time", end_time);
        write_string_field(&mut output, &mut fields, "type", "subsegment");

        if !self.annotations().is_empty() {
            write_separator(&mut output, &mut fields);
            write_json_string("annotations", &mut output);
            output.push(':');
            TraceValue::from(self.annotations().clone()).write_json(&mut output);
        }

        let mut metadata = self.metadata().clone();
        if let Some(response) = self.response() {
            metadata.insert("response".to_owned(), response.clone());
        }
        if let Some(error) = self.error() {
            metadata.insert("error".to_owned(), error.clone());
        }

        if !metadata.is_empty() {
            write_separator(&mut output, &mut fields);
            write_json_string("metadata", &mut output);
            output.push(':');
            write_metadata(
                self.service_name().unwrap_or("default"),
                metadata,
                &mut output,
            );
        }

        output.push('}');
        Ok(output)
    }
}

fn write_string_field(output: &mut String, fields: &mut usize, name: &str, value: &str) {
    write_separator(output, fields);
    write_json_string(name, output);
    output.push(':');
    write_json_string(value, output);
}

fn write_number_field(output: &mut String, fields: &mut usize, name: &str, value: f64) {
    write_separator(output, fields);
    write_json_string(name, output);
    output.push(':');
    output.push_str(&value.to_string());
}

fn write_metadata(namespace: &str, metadata: TraceFields, output: &mut String) {
    output.push('{');
    write_json_string(namespace, output);
    output.push(':');
    TraceValue::from(metadata).write_json(output);
    output.push('}');
}

fn write_separator(output: &mut String, fields: &mut usize) {
    if *fields > 0 {
        output.push(',');
    }
    *fields += 1;
}

#[cfg(test)]
mod tests {
    use crate::{TraceContext, TraceSegment, Tracer, TracerConfig, XrayDocumentError};

    #[test]
    fn renders_xray_subsegment_document() {
        let tracer = Tracer::with_config(TracerConfig::new("orders"));
        let context = TraceContext::new("handler")
            .with_trace_id("1-67891233-abcdef012345678912345678")
            .with_parent_id("53995c3f42cd8ad8");
        let segment = tracer
            .segment_with_context(context)
            .with_annotation("tenant", "north")
            .with_metadata("attempt", 2)
            .with_response("ok")
            .with_error("failed");

        let document = segment
            .to_xray_subsegment_document("70de5b6f19ff9a0a", 1_700_000_000.0, 1_700_000_001.25)
            .expect("document should render");

        assert_eq!(
            document,
            "{\"name\":\"handler\",\"id\":\"70de5b6f19ff9a0a\",\
             \"trace_id\":\"1-67891233-abcdef012345678912345678\",\
             \"parent_id\":\"53995c3f42cd8ad8\",\"start_time\":1700000000,\
             \"end_time\":1700000001.25,\"type\":\"subsegment\",\
             \"annotations\":{\"tenant\":\"north\"},\
             \"metadata\":{\"orders\":{\"attempt\":2,\"error\":\"failed\",\"response\":\"ok\"}}}"
        );
    }

    #[test]
    fn rejects_segments_without_xray_context() {
        let segment = TraceSegment::new(TraceContext::new("handler"));

        assert_eq!(
            segment.to_xray_subsegment_document("70de5b6f19ff9a0a", 1.0, 2.0),
            Err(XrayDocumentError::MissingTraceId)
        );

        let segment = TraceSegment::new(
            TraceContext::new("handler").with_trace_id("1-67891233-abcdef012345678912345678"),
        );

        assert_eq!(
            segment.to_xray_subsegment_document("70de5b6f19ff9a0a", 1.0, 2.0),
            Err(XrayDocumentError::MissingParentId)
        );
    }

    #[test]
    fn rejects_disabled_segments_empty_ids_and_invalid_times() {
        let context = TraceContext::new("handler")
            .with_trace_id("1-67891233-abcdef012345678912345678")
            .with_parent_id("53995c3f42cd8ad8");

        let disabled = TraceSegment::new(context.clone()).with_enabled(false);
        assert_eq!(
            disabled.to_xray_subsegment_document("70de5b6f19ff9a0a", 1.0, 2.0),
            Err(XrayDocumentError::Disabled)
        );

        let segment = TraceSegment::new(context);
        assert_eq!(
            segment.to_xray_subsegment_document("  ", 1.0, 2.0),
            Err(XrayDocumentError::EmptyId)
        );
        assert_eq!(
            segment.to_xray_subsegment_document("70de5b6f19ff9a0a", 2.0, 1.0),
            Err(XrayDocumentError::InvalidTimestamps)
        );
    }
}
