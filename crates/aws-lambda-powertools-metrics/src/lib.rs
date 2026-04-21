//! Metrics utility.

mod config;
mod metric;
mod metrics;

pub use config::MetricsConfig;
pub use metric::{Metric, MetricUnit};
pub use metrics::Metrics;
