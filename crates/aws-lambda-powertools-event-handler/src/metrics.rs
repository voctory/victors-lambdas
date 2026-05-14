//! HTTP metrics middleware.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use aws_lambda_powertools_metrics::{MetricUnit, Metrics, MetricsError};

use crate::{Request, Response};

const NOT_FOUND_ROUTE: &str = "NOT_FOUND";

#[derive(Clone, Copy, Debug)]
struct HttpMetricsStart(Instant);

impl HttpMetricsStart {
    fn now() -> Self {
        Self(Instant::now())
    }

    fn elapsed(self) -> Duration {
        self.0.elapsed()
    }
}

/// Result type returned when recording HTTP request metrics.
pub type HttpMetricsResult<'a> = Result<&'a mut Metrics, MetricsError>;

/// Builds request middleware that captures the HTTP request start time.
///
/// Pair this with [`http_metrics_response_middleware`] to emit per-request
/// metrics after route handling.
pub fn http_metrics_start_middleware() -> impl Fn(Request) -> Request + Clone + Send + Sync + 'static
{
    |request| request.with_extension(HttpMetricsStart::now())
}

/// Builds response middleware that emits per-request metrics and flushes them.
///
/// The middleware records `latency` in milliseconds plus `fault` and `error`
/// count metrics. It uses the matched route label as the `route` dimension, or
/// `NOT_FOUND` when the request did not match a route.
pub fn http_metrics_response_middleware(
    metrics: Arc<Mutex<Metrics>>,
) -> impl Fn(&Request, Response) -> Response + Clone + Send + Sync + 'static {
    move |request, response| {
        let latency = request
            .extension::<HttpMetricsStart>()
            .map_or(Duration::ZERO, |start| start.elapsed());

        if let Ok(mut metrics) = metrics.lock() {
            if record_http_metrics(&mut metrics, request, &response, latency).is_ok() {
                let _ = metrics.flush();
            }
        }

        response
    }
}

/// Records route, latency, error, and fault metrics for an HTTP response.
///
/// This function only mutates the provided collector; use
/// [`Metrics::flush`] or [`Metrics::write_to`] to emit the pending EMF event.
///
/// # Errors
///
/// Returns [`MetricsError`] when the collector rejects a metric, dimension, or
/// metadata value.
pub fn record_http_metrics<'a>(
    metrics: &'a mut Metrics,
    request: &Request,
    response: &Response,
    latency: Duration,
) -> HttpMetricsResult<'a> {
    let status_code = response.status_code();
    let route = request
        .matched_route()
        .map_or_else(|| NOT_FOUND_ROUTE.to_owned(), crate::MatchedRoute::label);

    metrics.add_metadata("httpMethod", request.method().as_str())?;
    metrics.add_metadata("path", request.path())?;
    metrics.add_metadata("statusCode", status_code.to_string())?;
    if let Some(user_agent) = request.header("user-agent") {
        metrics.add_metadata("userAgent", user_agent)?;
    }
    if let Some(ip_address) = forwarded_ip_address(request) {
        metrics.add_metadata("ipAddress", ip_address)?;
    }

    metrics.add_dimension("route", route)?;
    metrics.try_add_metric(
        "latency",
        latency.as_secs_f64() * 1000.0,
        MetricUnit::Milliseconds,
    )?;
    metrics.try_add_metric(
        "fault",
        if status_code >= 500 { 1.0 } else { 0.0 },
        MetricUnit::Count,
    )?;
    metrics.try_add_metric(
        "error",
        if (400..500).contains(&status_code) {
            1.0
        } else {
            0.0
        },
        MetricUnit::Count,
    )?;

    Ok(metrics)
}

fn forwarded_ip_address(request: &Request) -> Option<&str> {
    request
        .header("x-forwarded-for")?
        .split(',')
        .map(str::trim)
        .find(|ip_address| !ip_address.is_empty())
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use aws_lambda_powertools_metrics::{MetricUnit, Metrics, MetricsConfig};

    use super::{
        HttpMetricsStart, http_metrics_response_middleware, http_metrics_start_middleware,
        record_http_metrics,
    };
    use crate::{Method, Request, Response};

    fn test_metrics() -> Metrics {
        Metrics::with_config(MetricsConfig::new("checkout", "Orders"))
    }

    fn disabled_test_metrics() -> Metrics {
        Metrics::with_config(MetricsConfig::new("checkout", "Orders").with_disabled(true))
    }

    fn assert_metric_value(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn records_http_metrics_with_route_dimension_and_metadata() {
        let mut metrics = test_metrics();
        let mut request = Request::new(Method::Get, "/orders/order-1")
            .with_header("User-Agent", "test-agent")
            .with_header("X-Forwarded-For", "203.0.113.1, 203.0.113.2");
        request.set_matched_route(Method::Get, "/orders/{order_id}");
        let response = Response::new(404);

        record_http_metrics(&mut metrics, &request, &response, Duration::from_millis(17))
            .expect("http metrics should be recorded");

        assert_eq!(
            metrics.dimensions().get("route").map(String::as_str),
            Some("GET /orders/{order_id}")
        );
        assert_eq!(metrics.metadata().get("httpMethod"), Some(&"GET".into()));
        assert_eq!(
            metrics.metadata().get("path"),
            Some(&"/orders/order-1".into())
        );
        assert_eq!(metrics.metadata().get("statusCode"), Some(&"404".into()));
        assert_eq!(
            metrics.metadata().get("userAgent"),
            Some(&"test-agent".into())
        );
        assert_eq!(
            metrics.metadata().get("ipAddress"),
            Some(&"203.0.113.1".into())
        );
        assert_eq!(metrics.metrics().len(), 3);
        assert_eq!(metrics.metrics()[0].name(), "latency");
        assert_eq!(metrics.metrics()[0].unit(), MetricUnit::Milliseconds);
        assert_metric_value(metrics.metrics()[0].value(), 17.0);
        assert_eq!(metrics.metrics()[1].name(), "fault");
        assert_eq!(metrics.metrics()[1].unit(), MetricUnit::Count);
        assert_metric_value(metrics.metrics()[1].value(), 0.0);
        assert_eq!(metrics.metrics()[2].name(), "error");
        assert_eq!(metrics.metrics()[2].unit(), MetricUnit::Count);
        assert_metric_value(metrics.metrics()[2].value(), 1.0);
    }

    #[test]
    fn records_not_found_route_and_fault_statuses() {
        let mut metrics = test_metrics();
        let request = Request::new(Method::Post, "/missing");
        let response = Response::new(503);

        record_http_metrics(&mut metrics, &request, &response, Duration::ZERO)
            .expect("http metrics should be recorded");

        assert_eq!(
            metrics.dimensions().get("route").map(String::as_str),
            Some("NOT_FOUND")
        );
        assert_metric_value(metrics.metrics()[1].value(), 1.0);
        assert_metric_value(metrics.metrics()[2].value(), 0.0);
    }

    #[test]
    fn start_middleware_stores_latency_start_time() {
        let request = http_metrics_start_middleware()(Request::new(Method::Get, "/orders"));

        assert!(request.extension::<HttpMetricsStart>().is_some());
    }

    #[test]
    fn response_middleware_flushes_recorded_metrics() {
        let metrics = Arc::new(std::sync::Mutex::new(disabled_test_metrics()));
        let request = http_metrics_start_middleware()(Request::new(Method::Get, "/orders"));

        let response =
            http_metrics_response_middleware(Arc::clone(&metrics))(&request, Response::ok("ok"));

        assert_eq!(response.status_code(), 200);
        assert!(metrics.lock().expect("metrics lock").metrics().is_empty());
    }
}
