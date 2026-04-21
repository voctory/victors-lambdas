//! Trace context values.

/// Identifies the active trace segment or subsegment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceContext {
    name: String,
}

impl TraceContext {
    /// Creates trace context with a segment name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Returns the segment name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}
