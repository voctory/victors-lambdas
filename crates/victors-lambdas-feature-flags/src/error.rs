//! Feature flag errors.

/// High-level feature flag error category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeatureFlagErrorKind {
    /// Feature flag configuration could not be parsed.
    Configuration,
    /// A backing store failed while loading configuration.
    Store,
    /// A feature value could not be converted to the requested type.
    Transform,
}

/// Result type used by the feature flags utility.
pub type FeatureFlagResult<T> = Result<T, FeatureFlagError>;

/// Error returned by feature flag configuration, store, or value conversion operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureFlagError {
    kind: FeatureFlagErrorKind,
    feature: Option<String>,
    message: String,
}

impl FeatureFlagError {
    /// Creates an error for invalid feature flag configuration.
    #[must_use]
    pub fn configuration(message: impl Into<String>) -> Self {
        Self {
            kind: FeatureFlagErrorKind::Configuration,
            feature: None,
            message: message.into(),
        }
    }

    /// Creates an error for a store failure.
    #[must_use]
    pub fn store(message: impl Into<String>) -> Self {
        Self {
            kind: FeatureFlagErrorKind::Store,
            feature: None,
            message: message.into(),
        }
    }

    /// Creates an error for a feature value that cannot be converted.
    #[must_use]
    pub fn transform(feature: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: FeatureFlagErrorKind::Transform,
            feature: Some(feature.into()),
            message: message.into(),
        }
    }

    /// Returns the feature flag error category.
    #[must_use]
    pub const fn kind(&self) -> FeatureFlagErrorKind {
        self.kind
    }

    /// Returns the feature name associated with the error when available.
    #[must_use]
    pub fn feature(&self) -> Option<&str> {
        self.feature.as_deref()
    }

    /// Returns the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for FeatureFlagError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FeatureFlagError {}
