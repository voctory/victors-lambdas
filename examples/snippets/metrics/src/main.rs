//! Metrics snippet for documentation.

use std::error::Error;

use aws_lambda_powertools::prelude::{MetricResolution, MetricUnit, Metrics, MetricsConfig};

fn main() -> Result<(), Box<dyn Error>> {
    let mut metrics = Metrics::with_config(MetricsConfig::new("checkout", "Orders"));

    metrics.add_default_dimension("environment", "local")?;
    metrics.add_dimension("route", "POST /orders")?;
    metrics.add_metric("OrdersCreated", 1.0, MetricUnit::Count);
    metrics.add_metric_with_resolution(
        "HandlerLatency",
        42.0,
        MetricUnit::Milliseconds,
        MetricResolution::High,
    );
    metrics.add_metadata("request_id", "request-1")?;

    let mut output = Vec::new();
    metrics.write_to_with_timestamp(&mut output, 1_700_000_000_000)?;
    print!("{}", String::from_utf8(output)?);

    Ok(())
}
