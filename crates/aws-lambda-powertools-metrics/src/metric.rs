//! Metric values.

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
