//! Metric values.

use crate::{MetricsError, validation};

/// `CloudWatch` metric unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricUnit {
    /// Count unit.
    Count,
    /// Milliseconds unit.
    Milliseconds,
    /// Bytes unit.
    Bytes,
    /// No unit.
    None,
}

impl MetricUnit {
    /// Returns the EMF unit name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Count => "Count",
            Self::Milliseconds => "Milliseconds",
            Self::Bytes => "Bytes",
            Self::None => "None",
        }
    }
}

/// A metric data point.
#[derive(Clone, Debug, PartialEq)]
pub struct Metric {
    name: String,
    value: f64,
    unit: MetricUnit,
}

impl Metric {
    /// Creates a metric data point.
    #[must_use]
    pub fn new(name: impl Into<String>, value: f64, unit: MetricUnit) -> Self {
        Self {
            name: name.into(),
            value,
            unit,
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
        let name = name.into();
        validation::validate_metric_name(&name)?;
        if !value.is_finite() {
            return Err(MetricsError::InvalidMetricValue { name });
        }

        Ok(Self { name, value, unit })
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_units_render_emf_names() {
        assert_eq!(MetricUnit::Count.as_str(), "Count");
        assert_eq!(MetricUnit::Milliseconds.as_str(), "Milliseconds");
        assert_eq!(MetricUnit::Bytes.as_str(), "Bytes");
        assert_eq!(MetricUnit::None.as_str(), "None");
    }

    #[test]
    fn try_new_accepts_valid_metric() {
        let metric =
            Metric::try_new("Processed", 42.0, MetricUnit::Count).expect("metric should be valid");

        assert_eq!(metric.name(), "Processed");
        assert_eq!(metric.unit(), MetricUnit::Count);
        assert!((metric.value() - 42.0).abs() < f64::EPSILON);
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
