//! Metrics collector.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use aws_lambda_powertools_core::cold_start::ColdStart;

use crate::{MetadataValue, Metric, MetricUnit, MetricsConfig, MetricsError, validation};

const MAX_METRICS_PER_EVENT: usize = 100;
const MAX_DIMENSIONS_PER_METRIC: usize = 30;
static COLD_START: ColdStart = ColdStart::new();

/// Collects metrics before emission.
#[derive(Clone, Debug, PartialEq)]
pub struct Metrics {
    config: MetricsConfig,
    metrics: Vec<Metric>,
    dimensions: BTreeMap<String, String>,
    metadata: BTreeMap<String, MetadataValue>,
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
            dimensions: BTreeMap::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Adds a metric data point.
    ///
    /// # Panics
    ///
    /// Panics when the metric name or value is invalid, or when adding the
    /// metric would exceed the EMF metric value limit. Use
    /// [`try_add_metric`](Self::try_add_metric) to handle validation errors.
    pub fn add_metric(&mut self, name: impl Into<String>, value: f64, unit: MetricUnit) {
        self.try_add_metric(name, value, unit)
            .expect("metric data point must be valid");
    }

    /// Adds a validated metric data point.
    ///
    /// # Errors
    ///
    /// Returns [`MetricsError`] when the metric name or value is invalid, or
    /// when adding the metric would exceed the EMF metric value limit.
    pub fn try_add_metric(
        &mut self,
        name: impl Into<String>,
        value: f64,
        unit: MetricUnit,
    ) -> Result<&mut Self, MetricsError> {
        let metric = Metric::try_new(name, value, unit)?;
        self.ensure_metric_capacity(1)?;
        self.metrics.push(metric);
        Ok(self)
    }

    /// Adds or replaces a dimension.
    ///
    /// Dimensions are rendered as top-level EMF members and included in the
    /// `CloudWatch` metric directive. The service name from the metrics
    /// configuration is always rendered as the `service` dimension unless this
    /// method replaces it.
    ///
    /// # Errors
    ///
    /// Returns [`MetricsError`] when the dimension name is invalid or when the
    /// dimension set would exceed the `CloudWatch` limit.
    pub fn add_dimension(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<&mut Self, MetricsError> {
        let name = name.into();
        validation::validate_dimension_name(&name)?;
        self.ensure_dimension_capacity(&name)?;
        self.dimensions.insert(name, value.into());
        Ok(self)
    }

    /// Adds or replaces metadata.
    ///
    /// Metadata is rendered as a top-level EMF member, but is not listed as a
    /// `CloudWatch` dimension or metric.
    ///
    /// # Errors
    ///
    /// Returns [`MetricsError`] when the metadata key is invalid or the value
    /// cannot be represented in JSON.
    pub fn add_metadata(
        &mut self,
        name: impl Into<String>,
        value: impl Into<MetadataValue>,
    ) -> Result<&mut Self, MetricsError> {
        let name = name.into();
        validation::validate_metadata_name(&name)?;
        let value = value.into();
        if !value.is_valid() {
            return Err(MetricsError::InvalidMetadataValue { name });
        }

        self.metadata.insert(name, value);
        Ok(self)
    }

    /// Adds the `ColdStart` metric once per execution environment.
    ///
    /// Returns `true` when the metric was added and `false` after the first
    /// invocation has already been marked.
    ///
    /// # Errors
    ///
    /// Returns [`MetricsError`] when adding the metric would exceed the EMF
    /// metric value limit.
    pub fn add_cold_start_metric(&mut self) -> Result<bool, MetricsError> {
        self.add_cold_start_metric_with_tracker(&COLD_START)
    }

    /// Renders the pending metric set as JSON EMF.
    ///
    /// Returns `Ok(None)` when metrics are disabled or when no metrics have
    /// been added.
    ///
    /// # Errors
    ///
    /// Returns [`MetricsError`] when pending metric data cannot be represented
    /// as a valid EMF event.
    pub fn to_emf_json(&self) -> Result<Option<String>, MetricsError> {
        self.to_emf_json_at(current_time_millis()?)
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

    /// Returns configured dimensions.
    #[must_use]
    pub fn dimensions(&self) -> &BTreeMap<String, String> {
        &self.dimensions
    }

    /// Returns configured metadata.
    #[must_use]
    pub fn metadata(&self) -> &BTreeMap<String, MetadataValue> {
        &self.metadata
    }

    /// Clears pending metrics, dimensions, and metadata.
    pub fn clear(&mut self) {
        self.metrics.clear();
        self.dimensions.clear();
        self.metadata.clear();
    }

    fn ensure_metric_capacity(&self, additional: usize) -> Result<(), MetricsError> {
        let count = self.metrics.len() + additional;
        if count > MAX_METRICS_PER_EVENT {
            return Err(MetricsError::TooManyMetrics {
                count,
                max: MAX_METRICS_PER_EVENT,
            });
        }

        Ok(())
    }

    fn ensure_dimension_capacity(&self, added_name: &str) -> Result<(), MetricsError> {
        let non_service_dimensions = self
            .dimensions
            .keys()
            .filter(|name| name.as_str() != "service")
            .count();
        let adds_non_service_dimension =
            added_name != "service" && !self.dimensions.contains_key(added_name);
        let count = 1 + non_service_dimensions + usize::from(adds_non_service_dimension);

        if count > MAX_DIMENSIONS_PER_METRIC {
            return Err(MetricsError::TooManyDimensions {
                count,
                max: MAX_DIMENSIONS_PER_METRIC,
            });
        }

        Ok(())
    }

    fn add_cold_start_metric_with_tracker(
        &mut self,
        tracker: &ColdStart,
    ) -> Result<bool, MetricsError> {
        if !tracker.mark_invocation() {
            return Ok(false);
        }

        self.ensure_metric_capacity(1)?;
        self.metrics
            .push(Metric::try_new("ColdStart", 1.0, MetricUnit::Count)?);
        Ok(true)
    }

    fn to_emf_json_at(&self, timestamp_millis: u64) -> Result<Option<String>, MetricsError> {
        if self.config.disabled() || self.metrics.is_empty() {
            return Ok(None);
        }

        let dimensions = self.dimension_entries()?;
        let metric_values = self.metric_values()?;
        self.validate_name_conflicts(&dimensions, &metric_values)?;

        let mut output = String::new();
        output.push('{');
        push_json_string(&mut output, "_aws");
        output.push(':');
        self.write_aws_metadata(&mut output, timestamp_millis, &dimensions, &metric_values);

        for (name, value) in &dimensions {
            output.push(',');
            push_json_string(&mut output, name);
            output.push(':');
            push_json_string(&mut output, value);
        }

        for (name, value) in &self.metadata {
            output.push(',');
            push_json_string(&mut output, name);
            output.push(':');
            value.write_json(&mut output);
        }

        for (name, (_unit, values)) in metric_values {
            output.push(',');
            push_json_string(&mut output, name);
            output.push(':');
            write_metric_values(&mut output, &values);
        }

        output.push('}');
        Ok(Some(output))
    }

    fn dimension_entries(&self) -> Result<Vec<(&str, &str)>, MetricsError> {
        let mut entries = Vec::with_capacity(self.dimensions.len() + 1);
        let service_name = self.config.service().service_name();
        let mut service_value = service_name;

        for (name, value) in &self.dimensions {
            validation::validate_dimension_name(name)?;
            if name == "service" {
                service_value = value;
            } else {
                entries.push((name.as_str(), value.as_str()));
            }
        }

        entries.insert(0, ("service", service_value));
        if entries.len() > MAX_DIMENSIONS_PER_METRIC {
            return Err(MetricsError::TooManyDimensions {
                count: entries.len(),
                max: MAX_DIMENSIONS_PER_METRIC,
            });
        }

        Ok(entries)
    }

    fn metric_values(&self) -> Result<BTreeMap<&str, (MetricUnit, Vec<f64>)>, MetricsError> {
        if self.metrics.len() > MAX_METRICS_PER_EVENT {
            return Err(MetricsError::TooManyMetrics {
                count: self.metrics.len(),
                max: MAX_METRICS_PER_EVENT,
            });
        }

        let mut values = BTreeMap::new();
        for metric in &self.metrics {
            validation::validate_metric_name(metric.name())?;
            if !metric.value().is_finite() {
                return Err(MetricsError::InvalidMetricValue {
                    name: metric.name().to_owned(),
                });
            }

            let entry = values
                .entry(metric.name())
                .or_insert_with(|| (metric.unit(), Vec::new()));
            if entry.0 != metric.unit() {
                return Err(MetricsError::ConflictingMetricUnit {
                    name: metric.name().to_owned(),
                });
            }
            entry.1.push(metric.value());
        }

        Ok(values)
    }

    fn validate_name_conflicts(
        &self,
        dimensions: &[(&str, &str)],
        metrics: &BTreeMap<&str, (MetricUnit, Vec<f64>)>,
    ) -> Result<(), MetricsError> {
        let dimension_names = dimensions
            .iter()
            .map(|(name, _value)| *name)
            .collect::<BTreeSet<_>>();

        for metric_name in metrics.keys() {
            if dimension_names.contains(metric_name) {
                return Err(MetricsError::NameConflict {
                    name: (*metric_name).to_owned(),
                    first: "dimension",
                    second: "metric",
                });
            }
        }

        for (metadata_name, metadata_value) in &self.metadata {
            validation::validate_metadata_name(metadata_name)?;
            if !metadata_value.is_valid() {
                return Err(MetricsError::InvalidMetadataValue {
                    name: metadata_name.to_owned(),
                });
            }

            let metadata_name = metadata_name.as_str();
            if dimension_names.contains(metadata_name) {
                return Err(MetricsError::NameConflict {
                    name: metadata_name.to_owned(),
                    first: "dimension",
                    second: "metadata",
                });
            }

            if metrics.contains_key(metadata_name) {
                return Err(MetricsError::NameConflict {
                    name: metadata_name.to_owned(),
                    first: "metric",
                    second: "metadata",
                });
            }
        }

        Ok(())
    }

    fn write_aws_metadata(
        &self,
        output: &mut String,
        timestamp_millis: u64,
        dimensions: &[(&str, &str)],
        metric_values: &BTreeMap<&str, (MetricUnit, Vec<f64>)>,
    ) {
        output.push('{');
        push_json_string(output, "Timestamp");
        output.push(':');
        output.push_str(&timestamp_millis.to_string());
        output.push(',');
        push_json_string(output, "CloudWatchMetrics");
        output.push_str(":[{");
        push_json_string(output, "Namespace");
        output.push(':');
        push_json_string(output, self.config.namespace());
        output.push(',');
        push_json_string(output, "Dimensions");
        output.push_str(":[[");
        for (index, (name, _value)) in dimensions.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            push_json_string(output, name);
        }
        output.push_str("]],");
        push_json_string(output, "Metrics");
        output.push_str(":[");
        for (index, (name, (unit, _values))) in metric_values.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push('{');
            push_json_string(output, "Name");
            output.push(':');
            push_json_string(output, name);
            output.push(',');
            push_json_string(output, "Unit");
            output.push(':');
            push_json_string(output, unit.as_str());
            output.push('}');
        }
        output.push_str("]}]}");
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

fn current_time_millis() -> Result<u64, MetricsError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_error| MetricsError::TimeBeforeUnixEpoch)?;

    Ok(u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
}

pub(crate) fn push_json_string(output: &mut String, value: &str) {
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
            character if character.is_control() => {
                write!(output, "\\u{:04x}", u32::from(character))
                    .expect("writing to a String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

fn write_metric_values(output: &mut String, values: &[f64]) {
    if let [value] = values {
        output.push_str(&value.to_string());
        return;
    }

    output.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&value.to_string());
    }
    output.push(']');
}

#[cfg(test)]
mod tests {
    use aws_lambda_powertools_core::cold_start::ColdStart;

    use super::*;

    fn configured_metrics() -> Metrics {
        Metrics::with_config(MetricsConfig::new("checkout", "Orders"))
    }

    #[test]
    fn to_emf_json_renders_dimensions_metadata_and_grouped_metrics() {
        let mut metrics = configured_metrics();
        metrics
            .add_dimension("Operation", "CreateOrder")
            .expect("dimension should be valid");
        metrics
            .add_metadata("request_id", "abc-123")
            .expect("metadata should be valid");
        metrics
            .add_metadata("sample_rate", 0.25)
            .expect("metadata should be valid");
        metrics
            .try_add_metric("Latency", 12.5, MetricUnit::Milliseconds)
            .expect("metric should be valid");
        metrics
            .try_add_metric("Latency", 7.0, MetricUnit::Milliseconds)
            .expect("metric should be valid");
        metrics
            .try_add_metric("Processed", 1.0, MetricUnit::Count)
            .expect("metric should be valid");

        let output = metrics
            .to_emf_json_at(123_456_789)
            .expect("rendering should succeed")
            .expect("metrics should render");

        assert_eq!(
            output,
            "{\"_aws\":{\"Timestamp\":123456789,\"CloudWatchMetrics\":[{\"Namespace\":\"Orders\",\"Dimensions\":[[\"service\",\"Operation\"]],\"Metrics\":[{\"Name\":\"Latency\",\"Unit\":\"Milliseconds\"},{\"Name\":\"Processed\",\"Unit\":\"Count\"}]}]},\"service\":\"checkout\",\"Operation\":\"CreateOrder\",\"request_id\":\"abc-123\",\"sample_rate\":0.25,\"Latency\":[12.5,7],\"Processed\":1}"
        );
    }

    #[test]
    fn to_emf_json_uses_service_dimension_override() {
        let mut metrics = configured_metrics();
        metrics
            .add_dimension("service", "payments")
            .expect("service dimension override should be valid");
        metrics
            .try_add_metric("Processed", 1.0, MetricUnit::Count)
            .expect("metric should be valid");

        let output = metrics
            .to_emf_json_at(1)
            .expect("rendering should succeed")
            .expect("metrics should render");

        assert_eq!(
            output,
            "{\"_aws\":{\"Timestamp\":1,\"CloudWatchMetrics\":[{\"Namespace\":\"Orders\",\"Dimensions\":[[\"service\"]],\"Metrics\":[{\"Name\":\"Processed\",\"Unit\":\"Count\"}]}]},\"service\":\"payments\",\"Processed\":1}"
        );
    }

    #[test]
    fn to_emf_json_returns_none_when_empty_or_disabled() {
        let empty = configured_metrics();
        assert_eq!(
            empty.to_emf_json_at(1).expect("rendering should succeed"),
            None
        );

        let mut disabled =
            Metrics::with_config(MetricsConfig::new("checkout", "Orders").with_disabled(true));
        disabled
            .try_add_metric("Processed", 1.0, MetricUnit::Count)
            .expect("metric should be valid");

        assert_eq!(
            disabled
                .to_emf_json_at(1)
                .expect("disabled rendering should succeed"),
            None
        );
    }

    #[test]
    fn try_add_metric_validates_metric_before_capacity() {
        let mut metrics = configured_metrics();
        for index in 0..MAX_METRICS_PER_EVENT {
            metrics
                .try_add_metric(format!("Metric{index}"), 1.0, MetricUnit::Count)
                .expect("metric should fit");
        }

        let error = metrics
            .try_add_metric("_aws", f64::NAN, MetricUnit::Count)
            .expect_err("metric validity is checked before capacity");

        assert_eq!(
            error,
            MetricsError::InvalidMetricName {
                name: "_aws".to_owned(),
                reason: "is reserved for EMF metadata"
            }
        );
    }

    #[test]
    fn try_add_metric_rejects_more_than_100_metric_values() {
        let mut metrics = configured_metrics();
        for index in 0..MAX_METRICS_PER_EVENT {
            metrics
                .try_add_metric(format!("Metric{index}"), 1.0, MetricUnit::Count)
                .expect("metric should fit");
        }

        let error = metrics
            .try_add_metric("Overflow", 1.0, MetricUnit::Count)
            .expect_err("101st metric should fail");

        assert_eq!(
            error,
            MetricsError::TooManyMetrics {
                count: MAX_METRICS_PER_EVENT + 1,
                max: MAX_METRICS_PER_EVENT
            }
        );
    }

    #[test]
    fn to_emf_json_rejects_conflicting_metric_units() {
        let mut metrics = configured_metrics();
        metrics
            .try_add_metric("Processed", 1.0, MetricUnit::Count)
            .expect("metric should be valid");
        metrics
            .try_add_metric("Processed", 1.0, MetricUnit::Bytes)
            .expect("metric name is valid even with conflicting unit");

        let error = metrics
            .to_emf_json_at(1)
            .expect_err("rendering should reject conflicting units");

        assert_eq!(
            error,
            MetricsError::ConflictingMetricUnit {
                name: "Processed".to_owned()
            }
        );
    }

    #[test]
    fn to_emf_json_rejects_dimension_metric_name_conflict() {
        let mut metrics = configured_metrics();
        metrics
            .add_dimension("Operation", "CreateOrder")
            .expect("dimension should be valid");
        metrics
            .try_add_metric("Operation", 1.0, MetricUnit::Count)
            .expect("metric should be valid");

        let error = metrics
            .to_emf_json_at(1)
            .expect_err("rendering should reject top-level conflicts");

        assert_eq!(
            error,
            MetricsError::NameConflict {
                name: "Operation".to_owned(),
                first: "dimension",
                second: "metric"
            }
        );
    }

    #[test]
    fn to_emf_json_rejects_metric_metadata_name_conflict() {
        let mut metrics = configured_metrics();
        metrics
            .add_metadata("Processed", true)
            .expect("metadata should be valid");
        metrics
            .try_add_metric("Processed", 1.0, MetricUnit::Count)
            .expect("metric should be valid");

        let error = metrics
            .to_emf_json_at(1)
            .expect_err("rendering should reject top-level conflicts");

        assert_eq!(
            error,
            MetricsError::NameConflict {
                name: "Processed".to_owned(),
                first: "metric",
                second: "metadata"
            }
        );
    }

    #[test]
    fn add_metadata_rejects_non_finite_float_values() {
        let mut metrics = configured_metrics();

        let error = metrics
            .add_metadata("sample_rate", f64::INFINITY)
            .expect_err("infinity cannot be represented in JSON");

        assert_eq!(
            error,
            MetricsError::InvalidMetadataValue {
                name: "sample_rate".to_owned()
            }
        );
    }

    #[test]
    fn add_cold_start_metric_uses_tracker_once() {
        let tracker = ColdStart::new();
        let mut metrics = configured_metrics();

        assert!(
            metrics
                .add_cold_start_metric_with_tracker(&tracker)
                .expect("first cold start metric should be added")
        );
        assert!(
            !metrics
                .add_cold_start_metric_with_tracker(&tracker)
                .expect("second cold start call should succeed without metric")
        );

        let [metric] = metrics.metrics() else {
            panic!("expected exactly one cold start metric");
        };
        assert_eq!(metric.name(), "ColdStart");
        assert_eq!(metric.unit(), MetricUnit::Count);
        assert!((metric.value() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn add_cold_start_metric_consumes_tracker_when_capacity_fails() {
        let tracker = ColdStart::new();
        let mut full_metrics = configured_metrics();
        for index in 0..MAX_METRICS_PER_EVENT {
            full_metrics
                .try_add_metric(format!("Metric{index}"), 1.0, MetricUnit::Count)
                .expect("metric should fit");
        }

        let error = full_metrics
            .add_cold_start_metric_with_tracker(&tracker)
            .expect_err("cold start metric should not fit");
        assert_eq!(
            error,
            MetricsError::TooManyMetrics {
                count: MAX_METRICS_PER_EVENT + 1,
                max: MAX_METRICS_PER_EVENT
            }
        );

        let mut next_metrics = configured_metrics();
        assert!(
            !next_metrics
                .add_cold_start_metric_with_tracker(&tracker)
                .expect("second call should succeed without metric")
        );
        assert!(next_metrics.metrics().is_empty());
    }

    #[test]
    fn add_dimension_allows_29_custom_dimensions_plus_service() {
        let mut metrics = configured_metrics();
        for index in 0..(MAX_DIMENSIONS_PER_METRIC - 1) {
            metrics
                .add_dimension(format!("Dimension{index}"), "value")
                .expect("dimension should fit");
        }

        metrics
            .try_add_metric("Processed", 1.0, MetricUnit::Count)
            .expect("metric should be valid");
        let output = metrics
            .to_emf_json_at(1)
            .expect("rendering should succeed")
            .expect("metrics should render");

        assert!(output.contains("\"service\""));
        assert!(output.contains("\"Dimension28\""));
    }

    #[test]
    fn add_dimension_rejects_30_custom_dimensions_plus_service() {
        let mut metrics = configured_metrics();
        for index in 0..(MAX_DIMENSIONS_PER_METRIC - 1) {
            metrics
                .add_dimension(format!("Dimension{index}"), "value")
                .expect("dimension should fit");
        }

        let error = metrics
            .add_dimension("Overflow", "value")
            .expect_err("service plus 30 custom dimensions is invalid");

        assert_eq!(
            error,
            MetricsError::TooManyDimensions {
                count: MAX_DIMENSIONS_PER_METRIC + 1,
                max: MAX_DIMENSIONS_PER_METRIC
            }
        );
    }

    #[test]
    fn clear_removes_pending_metrics_dimensions_and_metadata() {
        let mut metrics = configured_metrics();
        metrics
            .add_dimension("Operation", "CreateOrder")
            .expect("dimension should be valid");
        metrics
            .add_metadata("request_id", "abc-123")
            .expect("metadata should be valid");
        metrics
            .try_add_metric("Processed", 1.0, MetricUnit::Count)
            .expect("metric should be valid");

        metrics.clear();

        assert!(metrics.metrics().is_empty());
        assert!(metrics.dimensions().is_empty());
        assert!(metrics.metadata().is_empty());
    }

    #[test]
    fn push_json_string_escapes_control_characters() {
        let mut output = String::new();

        push_json_string(&mut output, "quote\" slash\\ newline\n tab\t bell\u{07}");

        assert_eq!(
            output,
            "\"quote\\\" slash\\\\ newline\\n tab\\t bell\\u0007\""
        );
    }
}
