//! Fixture loading helpers.

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;

/// Error returned when a fixture cannot be loaded or decoded.
#[derive(Debug)]
pub enum FixtureError {
    /// The fixture file could not be read.
    Read {
        /// Fixture path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// The fixture file was not valid JSON for the requested type.
    Json {
        /// Fixture path that failed.
        path: PathBuf,
        /// Underlying JSON error.
        source: serde_json::Error,
    },
}

impl FixtureError {
    /// Returns the fixture path that failed.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Read { path, .. } | Self::Json { path, .. } => path,
        }
    }

    /// Returns true when the fixture failed while reading the file.
    #[must_use]
    pub const fn is_read_error(&self) -> bool {
        matches!(self, Self::Read { .. })
    }

    /// Returns true when the fixture failed while decoding JSON.
    #[must_use]
    pub const fn is_json_error(&self) -> bool {
        matches!(self, Self::Json { .. })
    }
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read fixture {}: {source}",
                    path.display()
                )
            }
            Self::Json { path, source } => {
                write!(
                    formatter,
                    "failed to decode JSON fixture {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for FixtureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
        }
    }
}

/// Reads a UTF-8 fixture file into a string.
///
/// # Errors
///
/// Returns an I/O error when the fixture cannot be read or is not valid UTF-8.
pub fn read_fixture(path: impl AsRef<Path>) -> io::Result<String> {
    std::fs::read_to_string(path)
}

/// Reads a fixture file into bytes.
///
/// # Errors
///
/// Returns an I/O error when the fixture cannot be read.
pub fn read_fixture_bytes(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

/// Reads and decodes a JSON fixture file.
///
/// # Errors
///
/// Returns a fixture error when the file cannot be read or when the JSON cannot
/// be decoded into `T`.
pub fn load_json_fixture<T>(path: impl AsRef<Path>) -> Result<T, FixtureError>
where
    T: DeserializeOwned,
{
    let path = path.as_ref();
    let bytes = read_fixture_bytes(path).map_err(|source| FixtureError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    serde_json::from_slice(&bytes).map_err(|source| FixtureError::Json {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde::Deserialize;

    use super::{FixtureError, load_json_fixture, read_fixture, read_fixture_bytes};

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct OrderFixture {
        order_id: String,
        quantity: u32,
    }

    #[test]
    fn fixture_readers_load_text_bytes_and_json() {
        let path = temp_fixture_path("order.json");
        fs::write(&path, r#"{"order_id":"order-1","quantity":2}"#)
            .expect("fixture should be written");

        let text = read_fixture(&path).expect("fixture should load as text");
        let bytes = read_fixture_bytes(&path).expect("fixture should load as bytes");
        let order = load_json_fixture::<OrderFixture>(&path).expect("fixture should decode");

        assert_eq!(text, r#"{"order_id":"order-1","quantity":2}"#);
        assert_eq!(bytes, br#"{"order_id":"order-1","quantity":2}"#);
        assert_eq!(
            order,
            OrderFixture {
                order_id: "order-1".to_owned(),
                quantity: 2,
            }
        );

        fs::remove_file(path).expect("fixture should be removed");
    }

    #[test]
    fn json_fixture_errors_include_failed_path() {
        let path = temp_fixture_path("broken.json");
        fs::write(&path, "{").expect("fixture should be written");

        let error = load_json_fixture::<OrderFixture>(&path).expect_err("invalid JSON should fail");

        assert!(matches!(error, FixtureError::Json { .. }));
        assert!(error.is_json_error());
        assert!(!error.is_read_error());
        assert_eq!(error.path(), path.as_path());

        fs::remove_file(path).expect("fixture should be removed");
    }

    fn temp_fixture_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("victors-lambdas-{nanos}-{name}"))
    }
}
