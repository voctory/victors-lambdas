//! Metric values.

use crate::{MetricsError, validation};

/// `CloudWatch` metric unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricUnit {
    /// Seconds unit.
    Seconds,
    /// Microseconds unit.
    Microseconds,
    /// Count unit.
    Count,
    /// Milliseconds unit.
    Milliseconds,
    /// Bytes unit.
    Bytes,
    /// Kilobytes unit.
    Kilobytes,
    /// Megabytes unit.
    Megabytes,
    /// Gigabytes unit.
    Gigabytes,
    /// Terabytes unit.
    Terabytes,
    /// Bits unit.
    Bits,
    /// Kilobits unit.
    Kilobits,
    /// Megabits unit.
    Megabits,
    /// Gigabits unit.
    Gigabits,
    /// Terabits unit.
    Terabits,
    /// Percent unit.
    Percent,
    /// Bytes per second unit.
    BytesPerSecond,
    /// Kilobytes per second unit.
    KilobytesPerSecond,
    /// Megabytes per second unit.
    MegabytesPerSecond,
    /// Gigabytes per second unit.
    GigabytesPerSecond,
    /// Terabytes per second unit.
    TerabytesPerSecond,
    /// Bits per second unit.
    BitsPerSecond,
    /// Kilobits per second unit.
    KilobitsPerSecond,
    /// Megabits per second unit.
    MegabitsPerSecond,
    /// Gigabits per second unit.
    GigabitsPerSecond,
    /// Terabits per second unit.
    TerabitsPerSecond,
    /// Count per second unit.
    CountPerSecond,
    /// No unit.
    NoUnit,
}

impl MetricUnit {
    /// Returns the EMF unit name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Seconds => "Seconds",
            Self::Microseconds => "Microseconds",
            Self::Count => "Count",
            Self::Milliseconds => "Milliseconds",
            Self::Bytes => "Bytes",
            Self::Kilobytes => "Kilobytes",
            Self::Megabytes => "Megabytes",
            Self::Gigabytes => "Gigabytes",
            Self::Terabytes => "Terabytes",
            Self::Bits => "Bits",
            Self::Kilobits => "Kilobits",
            Self::Megabits => "Megabits",
            Self::Gigabits => "Gigabits",
            Self::Terabits => "Terabits",
            Self::Percent => "Percent",
            Self::BytesPerSecond => "Bytes/Second",
            Self::KilobytesPerSecond => "Kilobytes/Second",
            Self::MegabytesPerSecond => "Megabytes/Second",
            Self::GigabytesPerSecond => "Gigabytes/Second",
            Self::TerabytesPerSecond => "Terabytes/Second",
            Self::BitsPerSecond => "Bits/Second",
            Self::KilobitsPerSecond => "Kilobits/Second",
            Self::MegabitsPerSecond => "Megabits/Second",
            Self::GigabitsPerSecond => "Gigabits/Second",
            Self::TerabitsPerSecond => "Terabits/Second",
            Self::CountPerSecond => "Count/Second",
            Self::NoUnit => "None",
        }
    }
}

/// `CloudWatch` metric storage resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricResolution {
    /// Standard one-minute resolution.
    Standard,
    /// High one-second resolution.
    High,
}

impl MetricResolution {
    /// Returns the `CloudWatch` resolution value.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        match self {
            Self::Standard => 60,
            Self::High => 1,
        }
    }

    pub(crate) const fn storage_resolution(self) -> Option<u16> {
        match self {
            Self::Standard => None,
            Self::High => Some(1),
        }
    }
}

/// A metric data point.
#[derive(Clone, Debug, PartialEq)]
pub struct Metric {
    name: String,
    value: f64,
    unit: MetricUnit,
    resolution: MetricResolution,
}

impl Metric {
    /// Creates a metric data point.
    #[must_use]
    pub fn new(name: impl Into<String>, value: f64, unit: MetricUnit) -> Self {
        Self {
            name: name.into(),
            value,
            unit,
            resolution: MetricResolution::Standard,
        }
    }

    /// Creates a validated metric data point.
    ///
    /// # Errors
    ///
    /// Returns [`MetricsError`] when the metric name is invalid or the value
    /// cannot be represented in JSON.
    pub fn try_new(
        name: impl Into<String>,
        value: f64,
        unit: MetricUnit,
    ) -> Result<Self, MetricsError> {
        Self::try_new_with_resolution(name, value, unit, MetricResolution::Standard)
    }

    /// Creates a validated metric data point with a storage resolution.
    ///
    /// # Errors
    ///
    /// Returns [`MetricsError`] when the metric name is invalid or the value
    /// cannot be represented in JSON.
    pub fn try_new_with_resolution(
        name: impl Into<String>,
        value: f64,
        unit: MetricUnit,
        resolution: MetricResolution,
    ) -> Result<Self, MetricsError> {
        let name = name.into();
        validation::validate_metric_name(&name)?;
        if !value.is_finite() {
            return Err(MetricsError::InvalidMetricValue { name });
        }

        Ok(Self {
            name,
            value,
            unit,
            resolution,
        })
    }

    /// Returns the metric name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the metric value.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the metric unit.
    #[must_use]
    pub fn unit(&self) -> MetricUnit {
        self.unit
    }

    /// Returns the metric storage resolution.
    #[must_use]
    pub fn resolution(&self) -> MetricResolution {
        self.resolution
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_units_render_emf_names() {
        assert_eq!(MetricUnit::Seconds.as_str(), "Seconds");
        assert_eq!(MetricUnit::Microseconds.as_str(), "Microseconds");
        assert_eq!(MetricUnit::Count.as_str(), "Count");
        assert_eq!(MetricUnit::Milliseconds.as_str(), "Milliseconds");
        assert_eq!(MetricUnit::Bytes.as_str(), "Bytes");
        assert_eq!(MetricUnit::Kilobytes.as_str(), "Kilobytes");
        assert_eq!(MetricUnit::Megabytes.as_str(), "Megabytes");
        assert_eq!(MetricUnit::Gigabytes.as_str(), "Gigabytes");
        assert_eq!(MetricUnit::Terabytes.as_str(), "Terabytes");
        assert_eq!(MetricUnit::Bits.as_str(), "Bits");
        assert_eq!(MetricUnit::Kilobits.as_str(), "Kilobits");
        assert_eq!(MetricUnit::Megabits.as_str(), "Megabits");
        assert_eq!(MetricUnit::Gigabits.as_str(), "Gigabits");
        assert_eq!(MetricUnit::Terabits.as_str(), "Terabits");
        assert_eq!(MetricUnit::Percent.as_str(), "Percent");
        assert_eq!(MetricUnit::BytesPerSecond.as_str(), "Bytes/Second");
        assert_eq!(MetricUnit::KilobytesPerSecond.as_str(), "Kilobytes/Second");
        assert_eq!(MetricUnit::MegabytesPerSecond.as_str(), "Megabytes/Second");
        assert_eq!(MetricUnit::GigabytesPerSecond.as_str(), "Gigabytes/Second");
        assert_eq!(MetricUnit::TerabytesPerSecond.as_str(), "Terabytes/Second");
        assert_eq!(MetricUnit::BitsPerSecond.as_str(), "Bits/Second");
        assert_eq!(MetricUnit::KilobitsPerSecond.as_str(), "Kilobits/Second");
        assert_eq!(MetricUnit::MegabitsPerSecond.as_str(), "Megabits/Second");
        assert_eq!(MetricUnit::GigabitsPerSecond.as_str(), "Gigabits/Second");
        assert_eq!(MetricUnit::TerabitsPerSecond.as_str(), "Terabits/Second");
        assert_eq!(MetricUnit::CountPerSecond.as_str(), "Count/Second");
        assert_eq!(MetricUnit::NoUnit.as_str(), "None");
    }

    #[test]
    fn metric_resolutions_render_cloudwatch_values() {
        assert_eq!(MetricResolution::Standard.as_u16(), 60);
        assert_eq!(MetricResolution::High.as_u16(), 1);
        assert_eq!(MetricResolution::Standard.storage_resolution(), None);
        assert_eq!(MetricResolution::High.storage_resolution(), Some(1));
    }

    #[test]
    fn try_new_accepts_valid_metric() {
        let metric =
            Metric::try_new("Processed", 42.0, MetricUnit::Count).expect("metric should be valid");

        assert_eq!(metric.name(), "Processed");
        assert_eq!(metric.unit(), MetricUnit::Count);
        assert_eq!(metric.resolution(), MetricResolution::Standard);
        assert!((metric.value() - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn try_new_with_resolution_accepts_high_resolution_metric() {
        let metric = Metric::try_new_with_resolution(
            "Latency",
            42.0,
            MetricUnit::Milliseconds,
            MetricResolution::High,
        )
        .expect("metric should be valid");

        assert_eq!(metric.resolution(), MetricResolution::High);
    }

    #[test]
    fn try_new_rejects_invalid_metric_name() {
        let error = Metric::try_new("_aws", 1.0, MetricUnit::Count).expect_err("_aws is reserved");

        assert_eq!(
            error,
            MetricsError::InvalidMetricName {
                name: "_aws".to_owned(),
                reason: "is reserved for EMF metadata"
            }
        );
    }

    #[test]
    fn try_new_rejects_non_finite_metric_value() {
        let error = Metric::try_new("Latency", f64::NAN, MetricUnit::Milliseconds)
            .expect_err("nan cannot be represented in JSON");

        assert_eq!(
            error,
            MetricsError::InvalidMetricValue {
                name: "Latency".to_owned()
            }
        );
    }
}
