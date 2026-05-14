//! Parser facade.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::ParseError;

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

    /// Parses a JSON string into the target payload type.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the input is not valid JSON or cannot be
    /// decoded into `T`.
    pub fn parse_json_str<T>(&self, payload: &str) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_str(payload)
            .map(ParsedEvent::new)
            .map_err(|error| ParseError::from_json_error(&error))
    }

    /// Parses JSON bytes into the target payload type.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the input is not valid JSON or cannot be
    /// decoded into `T`.
    pub fn parse_json_slice<T>(&self, payload: &[u8]) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_slice(payload)
            .map(ParsedEvent::new)
            .map_err(|error| ParseError::from_json_error(&error))
    }

    /// Decodes a JSON value into the target payload type.
    ///
    /// # Errors
    ///
    /// Returns a parse error when the value cannot be decoded into `T`.
    pub fn parse_json_value<T>(&self, payload: Value) -> Result<ParsedEvent<T>, ParseError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(payload)
            .map(ParsedEvent::new)
            .map_err(|error| ParseError::from_json_error(&error))
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use crate::{EventParser, ParseErrorKind};

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct OrderEvent {
        order_id: String,
        quantity: u32,
    }

    #[test]
    fn parses_json_string_into_payload() {
        let parsed = EventParser::new()
            .parse_json_str::<OrderEvent>(r#"{"order_id":"order-1","quantity":2}"#)
            .expect("valid event parses");

        assert_eq!(
            parsed.into_payload(),
            OrderEvent {
                order_id: String::from("order-1"),
                quantity: 2,
            }
        );
    }

    #[test]
    fn parses_json_slice_into_payload() {
        let parsed = EventParser::new()
            .parse_json_slice::<OrderEvent>(br#"{"order_id":"order-1","quantity":2}"#)
            .expect("valid event parses");

        assert_eq!(parsed.payload().quantity, 2);
    }

    #[test]
    fn parses_json_value_into_payload() {
        let parsed = EventParser::new()
            .parse_json_value::<OrderEvent>(json!({
                "order_id": "order-1",
                "quantity": 2,
            }))
            .expect("valid event parses");

        assert_eq!(parsed.payload().order_id, "order-1");
    }

    #[test]
    fn returns_data_error_for_schema_mismatch() {
        let error = EventParser::new()
            .parse_json_str::<OrderEvent>(r#"{"order_id":"order-1","quantity":"many"}"#)
            .expect_err("invalid event should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert_eq!(error.line(), Some(1));
        assert!(error.column().is_some());
        assert!(error.message().contains("invalid type"));
    }

    #[test]
    fn returns_eof_error_for_incomplete_json() {
        let error = EventParser::new()
            .parse_json_str::<OrderEvent>(r#"{"order_id":"order-1""#)
            .expect_err("incomplete JSON should fail");

        assert_eq!(error.kind(), ParseErrorKind::Eof);
        assert_eq!(error.line(), Some(1));
        assert!(error.column().is_some());
    }
}
