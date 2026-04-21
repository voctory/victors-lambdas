//! Trace attribute values.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A deterministic map of trace fields.
pub type TraceFields = BTreeMap<String, TraceValue>;

/// A JSON-compatible value attached to trace metadata or annotations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceValue {
    kind: TraceValueKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TraceValueKind {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<TraceValue>),
    Object(TraceFields),
}

impl TraceValue {
    /// Creates a JSON null value.
    #[must_use]
    pub const fn null() -> Self {
        Self {
            kind: TraceValueKind::Null,
        }
    }

    /// Creates a JSON boolean value.
    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        Self {
            kind: TraceValueKind::Bool(value),
        }
    }

    /// Creates a JSON signed integer value.
    #[must_use]
    pub fn number_i128(value: i128) -> Self {
        Self::number_string(value.to_string())
    }

    /// Creates a JSON unsigned integer value.
    #[must_use]
    pub fn number_u128(value: u128) -> Self {
        Self::number_string(value.to_string())
    }

    /// Creates a JSON floating point value.
    ///
    /// Non-finite values are represented as JSON null because JSON has no
    /// portable representation for `NaN` or infinity.
    #[must_use]
    pub fn number_f64(value: f64) -> Self {
        if value.is_finite() {
            Self::number_string(value.to_string())
        } else {
            Self::null()
        }
    }

    /// Creates a JSON string value.
    #[must_use]
    pub fn string(value: impl Into<String>) -> Self {
        Self {
            kind: TraceValueKind::String(value.into()),
        }
    }

    /// Creates a JSON array value.
    #[must_use]
    pub fn array<I, V>(values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<TraceValue>,
    {
        Self {
            kind: TraceValueKind::Array(values.into_iter().map(Into::into).collect()),
        }
    }

    /// Creates a JSON object value.
    ///
    /// Blank field names are ignored.
    #[must_use]
    pub fn object<I, K, V>(fields: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<TraceValue>,
    {
        let mut object = TraceFields::new();
        for (key, value) in fields {
            if let Some(key) = normalize_key(key) {
                object.insert(key, value.into());
            }
        }

        Self {
            kind: TraceValueKind::Object(object),
        }
    }

    /// Renders this value as valid JSON.
    #[must_use]
    pub fn to_json_string(&self) -> String {
        let mut output = String::new();
        self.write_json(&mut output);
        output
    }

    pub(crate) fn write_json(&self, output: &mut String) {
        match &self.kind {
            TraceValueKind::Null => output.push_str("null"),
            TraceValueKind::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            TraceValueKind::Number(value) => output.push_str(value),
            TraceValueKind::String(value) => write_json_string(value, output),
            TraceValueKind::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    value.write_json(output);
                }
                output.push(']');
            }
            TraceValueKind::Object(fields) => {
                output.push('{');
                for (index, (key, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    write_json_string(key, output);
                    output.push(':');
                    value.write_json(output);
                }
                output.push('}');
            }
        }
    }

    fn number_string(value: String) -> Self {
        Self {
            kind: TraceValueKind::Number(value),
        }
    }
}

impl From<()> for TraceValue {
    fn from((): ()) -> Self {
        Self::null()
    }
}

impl From<bool> for TraceValue {
    fn from(value: bool) -> Self {
        Self::boolean(value)
    }
}

impl From<&str> for TraceValue {
    fn from(value: &str) -> Self {
        Self::string(value)
    }
}

impl From<&String> for TraceValue {
    fn from(value: &String) -> Self {
        Self::string(value)
    }
}

impl From<String> for TraceValue {
    fn from(value: String) -> Self {
        Self::string(value)
    }
}

impl<T> From<Option<T>> for TraceValue
where
    T: Into<TraceValue>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or_else(Self::null, Into::into)
    }
}

impl<T> From<Vec<T>> for TraceValue
where
    T: Into<TraceValue>,
{
    fn from(value: Vec<T>) -> Self {
        Self::array(value)
    }
}

impl From<TraceFields> for TraceValue {
    fn from(value: TraceFields) -> Self {
        Self {
            kind: TraceValueKind::Object(value),
        }
    }
}

macro_rules! impl_signed_number {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for TraceValue {
                fn from(value: $ty) -> Self {
                    Self::number_string(value.to_string())
                }
            }
        )*
    };
}

macro_rules! impl_unsigned_number {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for TraceValue {
                fn from(value: $ty) -> Self {
                    Self::number_string(value.to_string())
                }
            }
        )*
    };
}

impl_signed_number!(i8, i16, i32, i64, i128, isize);
impl_unsigned_number!(u8, u16, u32, u64, u128, usize);

impl From<f32> for TraceValue {
    fn from(value: f32) -> Self {
        Self::number_f64(f64::from(value))
    }
}

impl From<f64> for TraceValue {
    fn from(value: f64) -> Self {
        Self::number_f64(value)
    }
}

pub(crate) fn normalize_key(key: impl Into<String>) -> Option<String> {
    let key = key.into();
    let key = key.trim();

    if key.is_empty() {
        None
    } else {
        Some(key.to_owned())
    }
}

fn write_json_string(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1f}' => {
                write!(output, "\\u{:04x}", u32::from(character))
                    .expect("writing to a String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::{TraceFields, TraceValue};

    #[test]
    fn renders_escaped_json_values() {
        let mut fields = TraceFields::new();
        fields.insert("enabled".to_owned(), true.into());
        fields.insert("message".to_owned(), "quoted \"line\"\nnext".into());
        fields.insert("nothing".to_owned(), TraceValue::null());

        assert_eq!(
            TraceValue::from(fields).to_json_string(),
            "{\"enabled\":true,\"message\":\"quoted \\\"line\\\"\\nnext\",\"nothing\":null}"
        );
    }

    #[test]
    fn renders_arrays_and_non_finite_numbers() {
        let value = TraceValue::array([1.5, f64::NAN]);

        assert_eq!(value.to_json_string(), "[1.5,null]");
    }

    #[test]
    fn ignores_blank_object_keys() {
        let value = TraceValue::object([("  ", "ignored"), ("kept", "value")]);

        assert_eq!(value.to_json_string(), "{\"kept\":\"value\"}");
    }
}
