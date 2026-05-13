//! Basic example binary for workspace validation.

use std::{
    error::Error,
    time::{Duration, SystemTime},
};

use aws_lambda_powertools::prelude::{
    BatchProcessor, BatchRecord, CachePolicy, EventParser, IdempotencyConfig, IdempotencyKey,
    IdempotencyRecord, IdempotencyStore, InMemoryIdempotencyStore, InMemoryParameterProvider,
    LogLevel, Logger, LoggerConfig, Method, MetricResolution, MetricUnit, Metrics, MetricsConfig,
    Parameters, Request, Response, Router, ServiceConfig, Tracer, TracerConfig, Validate,
    ValidationResult, Validator,
};

struct Order {
    id: &'static str,
    quantity: i64,
}

impl Validate for Order {
    fn validate(&self, validator: &Validator) -> ValidationResult {
        validator.required_text_field("id", self.id)?;
        validator.i64_in_range("quantity", self.quantity, 1, 10)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let service = ServiceConfig::new("basic-lambda");
    let logger =
        Logger::with_config(LoggerConfig::new(service.service_name()).with_level(LogLevel::Info));

    let order = EventParser::new()
        .parse(Order {
            id: "order-1",
            quantity: 1,
        })
        .into_payload();
    Validator::new().validate(&order)?;

    let parameters = Parameters::with_cache_policy(
        InMemoryParameterProvider::new().with_parameter("/basic-lambda/table", "orders"),
        CachePolicy::forever(),
    );
    let table = parameters.get("/basic-lambda/table").map_or_else(
        || String::from("missing"),
        |parameter| parameter.value().to_owned(),
    );

    let records = [
        BatchRecord::new("message-1", order.quantity),
        BatchRecord::new("message-2", 0),
    ];
    let report = BatchProcessor::new().process(&records, |record| {
        if *record.payload() > 0 {
            Ok(())
        } else {
            Err("quantity must be positive")
        }
    });
    let batch_response = report.response();

    let mut store = InMemoryIdempotencyStore::new();
    let key = IdempotencyKey::from("request-1");
    let record = IdempotencyRecord::completed_until(
        key.clone(),
        SystemTime::now() + Duration::from_secs(60),
    );
    let _ = store.put(record)?;
    let idempotent = store.get(&key)?.is_some();
    let idempotency_disabled = IdempotencyConfig::new(false).disabled();

    let mut router = Router::new();
    router.post("/orders/{id}", |request| {
        Response::ok(request.path_param("id").unwrap_or("missing"))
    });
    let route_response = router.handle(Request::new(Method::Post, "/orders/order-1"));

    let tracer = Tracer::with_config(TracerConfig::new(service.service_name()));
    let mut segment = tracer
        .segment("handler")
        .with_annotation("route", router.routes()[0].path());
    segment.capture_response("accepted");

    let mut metrics = Metrics::with_config(MetricsConfig::new(service.service_name(), "Example"));
    metrics.add_default_dimension("environment", "local")?;
    metrics.add_metric("OrdersProcessed", 1.0, MetricUnit::Count);
    metrics.add_metric_with_resolution(
        "HandlerLatency",
        42.0,
        MetricUnit::Milliseconds,
        MetricResolution::High,
    );
    metrics.add_dimension("route", router.routes()[0].path())?;
    metrics.add_metadata("table", table.as_str())?;
    metrics.add_metadata("idempotent", idempotent)?;

    let log_line = logger
        .info("handled order")
        .field("order_id", order.id)
        .field("table", table.as_str())
        .field("route_count", router.routes().len())
        .field("route_status", route_response.status_code())
        .field("failed_items", batch_response.batch_item_failures().len())
        .field("idempotent", idempotent)
        .field("idempotency_disabled", idempotency_disabled)
        .field("trace", segment.name())
        .render()
        .unwrap_or_default();
    println!("{log_line}");

    metrics.flush()?;

    Ok(())
}
