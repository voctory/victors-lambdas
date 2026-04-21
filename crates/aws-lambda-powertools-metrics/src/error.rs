//! Metrics errors.

/// Error returned when metrics validation or rendering fails.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetricsError {
    /// A metric name is not valid for `CloudWatch` EMF.
    InvalidMetricName {
        /// Metric name that failed validation.
        name: String,
        /// Human-readable validation reason.
        reason: &'static str,
    },
    /// A metric value cannot be represented in JSON EMF.
    InvalidMetricValue {
        /// Metric name associated with the invalid value.
        name: String,
    },
    /// A dimension name is not valid for `CloudWatch` EMF.
    InvalidDimensionName {
        /// Dimension name that failed validation.
        name: String,
        /// Human-readable validation reason.
        reason: &'static str,
    },
    /// A metadata key cannot be rendered as a top-level EMF member.
    InvalidMetadataName {
        /// Metadata key that failed validation.
        name: String,
        /// Human-readable validation reason.
        reason: &'static str,
    },
    /// A metadata value cannot be represented in JSON.
    InvalidMetadataValue {
        /// Metadata key associated with the invalid value.
        name: String,
    },
    /// A metric name was used with more than one unit in the same EMF event.
    ConflictingMetricUnit {
        /// Metric name with conflicting unit definitions.
        name: String,
    },
    /// A top-level EMF member name is used by more than one category.
    NameConflict {
        /// Conflicting top-level member name.
        name: String,
        /// First category using the name.
        first: &'static str,
        /// Second category using the name.
        second: &'static str,
    },
    /// The EMF event contains more metrics than `CloudWatch` accepts.
    TooManyMetrics {
        /// Number of metric values in the event.
        count: usize,
        /// Maximum supported metric values per event.
        max: usize,
    },
    /// The EMF event contains more dimensions than `CloudWatch` accepts.
    TooManyDimensions {
        /// Number of dimension keys in the event.
        count: usize,
        /// Maximum supported dimension keys per metric.
        max: usize,
    },
    /// The system clock is earlier than the Unix epoch.
    TimeBeforeUnixEpoch,
}

impl std::fmt::Display for MetricsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMetricName { name, reason } => {
                write!(formatter, "invalid metric name {name:?}: {reason}")
            }
            Self::InvalidMetricValue { name } => {
                write!(
                    formatter,
                    "invalid metric value for {name:?}: value must be finite"
                )
            }
            Self::InvalidDimensionName { name, reason } => {
                write!(formatter, "invalid dimension name {name:?}: {reason}")
            }
            Self::InvalidMetadataName { name, reason } => {
                write!(formatter, "invalid metadata name {name:?}: {reason}")
            }
            Self::InvalidMetadataValue { name } => {
                write!(
                    formatter,
                    "invalid metadata value for {name:?}: value must be finite"
                )
            }
            Self::ConflictingMetricUnit { name } => {
                write!(
                    formatter,
                    "metric {name:?} cannot use multiple units in one EMF event"
                )
            }
            Self::NameConflict {
                name,
                first,
                second,
            } => write!(
                formatter,
                "top-level EMF member {name:?} is used as both {first} and {second}"
            ),
            Self::TooManyMetrics { count, max } => write!(
                formatter,
                "too many metric values for one EMF event: {count} exceeds {max}"
            ),
            Self::TooManyDimensions { count, max } => write!(
                formatter,
                "too many dimensions for one EMF event: {count} exceeds {max}"
            ),
            Self::TimeBeforeUnixEpoch => formatter.write_str("system clock is before Unix epoch"),
        }
    }
}

impl std::error::Error for MetricsError {}
