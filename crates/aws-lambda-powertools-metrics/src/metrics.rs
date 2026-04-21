//! Metrics collector.

use crate::{Metric, MetricUnit, MetricsConfig};

/// Collects metrics before emission.
#[derive(Clone, Debug, PartialEq)]
pub struct Metrics {
    config: MetricsConfig,
    metrics: Vec<Metric>,
}

impl Metrics {
    /// Creates metrics from environment configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(MetricsConfig::from_env())
    }

    /// Creates metrics with explicit configuration.
    #[must_use]
    pub fn with_config(config: MetricsConfig) -> Self {
        Self {
            config,
            metrics: Vec::new(),
        }
    }

    /// Adds a metric data point.
    pub fn add_metric(&mut self, name: impl Into<String>, value: f64, unit: MetricUnit) {
        self.metrics.push(Metric::new(name, value, unit));
    }

    /// Returns the metrics configuration.
    #[must_use]
    pub fn config(&self) -> &MetricsConfig {
        &self.config
    }

    /// Returns pending metric data points.
    #[must_use]
    pub fn metrics(&self) -> &[Metric] {
        &self.metrics
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
