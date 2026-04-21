//! Parser facade.

/// Parsed Lambda event payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedEvent<T> {
    payload: T,
}

impl<T> ParsedEvent<T> {
    /// Creates a parsed event wrapper.
    #[must_use]
    pub fn new(payload: T) -> Self {
        Self { payload }
    }

    /// Returns the parsed payload.
    #[must_use]
    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// Consumes the wrapper and returns the parsed payload.
    #[must_use]
    pub fn into_payload(self) -> T {
        self.payload
    }
}

/// Parser facade for event envelopes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventParser;

impl EventParser {
    /// Creates an event parser.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Wraps an already-decoded payload as a parsed event.
    #[must_use]
    pub fn parse<T>(&self, payload: T) -> ParsedEvent<T> {
        ParsedEvent::new(payload)
    }
}
