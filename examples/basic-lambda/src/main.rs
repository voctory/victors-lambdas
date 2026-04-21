//! Basic example binary for workspace validation.

use aws_lambda_powertools::prelude::{LogLevel, LoggerConfig, Metrics, Tracer};
use aws_lambda_powertools::{logger::Logger, metrics::MetricUnit};

fn main() {
    let logger = Logger::with_config(LoggerConfig::new("example").with_level(LogLevel::Info));
    let tracer = Tracer::new();
    let _context = tracer.context("handler");

    let mut metrics = Metrics::new();
    metrics.add_metric("ColdStart", 1.0, MetricUnit::Count);

    println!(
        "service={} level={:?} metrics={}",
        logger.service_name(),
        logger.level(),
        metrics.metrics().len()
    );
}
