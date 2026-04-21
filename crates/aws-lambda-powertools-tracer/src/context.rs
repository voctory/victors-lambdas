//! Trace context values.

/// Identifies the active trace segment or subsegment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceContext {
    name: String,
    trace_id: Option<String>,
    parent_id: Option<String>,
    sampled: Option<bool>,
}

impl TraceContext {
    /// Creates trace context with a segment name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            trace_id: None,
            parent_id: None,
            sampled: None,
        }
    }

    /// Creates trace context from an AWS X-Ray trace header.
    ///
    /// Supported header fields are `Root`, `Parent`, and `Sampled`. Unknown
    /// fields are ignored so callers can pass the full `X-Amzn-Trace-Id`
    /// header value.
    #[must_use]
    pub fn from_xray_header(name: impl Into<String>, header: &str) -> Self {
        let mut context = Self::new(name);

        for part in header.split(';') {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };

            match key.trim() {
                "Root" => {
                    if let Some(trace_id) = normalize_identifier(value) {
                        context.trace_id = Some(trace_id);
                    }
                }
                "Parent" => {
                    if let Some(parent_id) = normalize_identifier(value) {
                        context.parent_id = Some(parent_id);
                    }
                }
                "Sampled" => {
                    if let Some(sampled) = parse_sampled(value) {
                        context.sampled = Some(sampled);
                    }
                }
                _ => {}
            }
        }

        context
    }

    /// Returns a copy of this context with an explicit trace identifier.
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = normalize_identifier(trace_id);
        self
    }

    /// Returns a copy of this context with an explicit parent segment identifier.
    #[must_use]
    pub fn with_parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = normalize_identifier(parent_id);
        self
    }

    /// Returns a copy of this context with an explicit sampling decision.
    #[must_use]
    pub const fn with_sampled(mut self, sampled: bool) -> Self {
        self.sampled = Some(sampled);
        self
    }

    /// Returns the segment name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the trace identifier, if one is known.
    #[must_use]
    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id.as_deref()
    }

    /// Returns the parent segment identifier, if one is known.
    #[must_use]
    pub fn parent_id(&self) -> Option<&str> {
        self.parent_id.as_deref()
    }

    /// Returns the sampling decision, if one is known.
    #[must_use]
    pub const fn sampled(&self) -> Option<bool> {
        self.sampled
    }
}

fn normalize_identifier(value: impl Into<String>) -> Option<String> {
    let value = value.into();
    let value = value.trim();

    (!value.is_empty()).then(|| value.to_owned())
}

fn parse_sampled(value: &str) -> Option<bool> {
    match value.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::TraceContext;

    #[test]
    fn new_stores_name_without_remote_identifiers() {
        let context = TraceContext::new("handler");

        assert_eq!(context.name(), "handler");
        assert_eq!(context.trace_id(), None);
        assert_eq!(context.parent_id(), None);
        assert_eq!(context.sampled(), None);
    }

    #[test]
    fn builder_methods_attach_remote_identifiers() {
        let context = TraceContext::new("handler")
            .with_trace_id(" 1-67891233-abcdef012345678912345678 ")
            .with_parent_id(" 53995c3f42cd8ad8 ")
            .with_sampled(true);

        assert_eq!(
            context.trace_id(),
            Some("1-67891233-abcdef012345678912345678")
        );
        assert_eq!(context.parent_id(), Some("53995c3f42cd8ad8"));
        assert_eq!(context.sampled(), Some(true));
    }

    #[test]
    fn from_xray_header_extracts_supported_fields() {
        let context = TraceContext::from_xray_header(
            "handler",
            "Root=1-67891233-abcdef012345678912345678;\
             Parent=53995c3f42cd8ad8;Sampled=1;Lineage=ignored",
        );

        assert_eq!(context.name(), "handler");
        assert_eq!(
            context.trace_id(),
            Some("1-67891233-abcdef012345678912345678")
        );
        assert_eq!(context.parent_id(), Some("53995c3f42cd8ad8"));
        assert_eq!(context.sampled(), Some(true));
    }

    #[test]
    fn from_xray_header_ignores_empty_and_unknown_values() {
        let context = TraceContext::from_xray_header(
            "handler",
            "Root= ;Parent=parent;Sampled=?;Malformed;Unknown=value",
        );

        assert_eq!(context.trace_id(), None);
        assert_eq!(context.parent_id(), Some("parent"));
        assert_eq!(context.sampled(), None);
    }

    #[test]
    fn from_xray_header_keeps_valid_values_when_later_duplicates_are_invalid() {
        let context = TraceContext::from_xray_header(
            "handler",
            "Root=1-67891233-abcdef012345678912345678;Root= ;\
             Parent=53995c3f42cd8ad8;Parent= ;Sampled=1;Sampled=?",
        );

        assert_eq!(
            context.trace_id(),
            Some("1-67891233-abcdef012345678912345678")
        );
        assert_eq!(context.parent_id(), Some("53995c3f42cd8ad8"));
        assert_eq!(context.sampled(), Some(true));
    }
}
