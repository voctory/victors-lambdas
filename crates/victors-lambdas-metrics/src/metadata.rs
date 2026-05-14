//! Metadata values.

/// Extra top-level EMF metadata value.
#[derive(Clone, Debug, PartialEq)]
pub enum MetadataValue {
    /// JSON string value.
    String(String),
    /// JSON signed integer value.
    Signed(i64),
    /// JSON unsigned integer value.
    Unsigned(u64),
    /// JSON floating point value.
    Float(f64),
    /// JSON boolean value.
    Bool(bool),
    /// JSON null value.
    Null,
}

impl MetadataValue {
    pub(crate) fn is_valid(&self) -> bool {
        match self {
            Self::Float(value) => value.is_finite(),
            Self::String(_) | Self::Signed(_) | Self::Unsigned(_) | Self::Bool(_) | Self::Null => {
                true
            }
        }
    }

    pub(crate) fn write_json(&self, output: &mut String) {
        match self {
            Self::String(value) => super::metrics::push_json_string(output, value),
            Self::Signed(value) => output.push_str(&value.to_string()),
            Self::Unsigned(value) => output.push_str(&value.to_string()),
            Self::Float(value) => output.push_str(&value.to_string()),
            Self::Bool(true) => output.push_str("true"),
            Self::Bool(false) => output.push_str("false"),
            Self::Null => output.push_str("null"),
        }
    }
}

impl From<String> for MetadataValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for MetadataValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<i64> for MetadataValue {
    fn from(value: i64) -> Self {
        Self::Signed(value)
    }
}

impl From<i32> for MetadataValue {
    fn from(value: i32) -> Self {
        Self::Signed(i64::from(value))
    }
}

impl From<u64> for MetadataValue {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<u32> for MetadataValue {
    fn from(value: u32) -> Self {
        Self::Unsigned(u64::from(value))
    }
}

impl From<f64> for MetadataValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<bool> for MetadataValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<()> for MetadataValue {
    fn from((): ()) -> Self {
        Self::Null
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_values_are_valid_json_metadata() {
        assert!(MetadataValue::from("request-id").is_valid());
        assert!(MetadataValue::from(1_i64).is_valid());
        assert!(MetadataValue::from(1_u64).is_valid());
        assert!(MetadataValue::from(0.25).is_valid());
        assert!(MetadataValue::from(true).is_valid());
        assert!(MetadataValue::from(()).is_valid());
    }

    #[test]
    fn non_finite_float_metadata_is_invalid() {
        assert!(!MetadataValue::from(f64::NAN).is_valid());
        assert!(!MetadataValue::from(f64::INFINITY).is_valid());
        assert!(!MetadataValue::from(f64::NEG_INFINITY).is_valid());
    }

    #[test]
    fn write_json_renders_supported_values() {
        let mut output = String::new();

        MetadataValue::from("quote\" newline\n").write_json(&mut output);
        assert_eq!(output, "\"quote\\\" newline\\n\"");

        output.clear();
        MetadataValue::from(5_i64).write_json(&mut output);
        assert_eq!(output, "5");

        output.clear();
        MetadataValue::from(false).write_json(&mut output);
        assert_eq!(output, "false");
    }
}
