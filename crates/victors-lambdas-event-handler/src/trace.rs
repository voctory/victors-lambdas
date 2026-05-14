//! HTTP trace record middleware.

use std::sync::Arc;

use victors_lambdas_core::cold_start;
use victors_lambdas_tracer::{TraceFields, TraceSegment, TraceValue, Tracer};

use crate::{Request, Response};

const NOT_FOUND_ROUTE: &str = "NOT_FOUND";

/// Function signature used to consume completed HTTP trace records.
pub type HttpTraceSink = dyn Fn(TraceSegment) + Send + Sync + 'static;

/// HTTP trace record configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpTraceConfig {
    capture_response: bool,
}

impl HttpTraceConfig {
    /// Creates a trace configuration with JSON response capture enabled.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            capture_response: true,
        }
    }

    /// Returns whether JSON responses should be captured.
    #[must_use]
    pub const fn capture_response(&self) -> bool {
        self.capture_response
    }

    /// Returns a copy with JSON response capture enabled or disabled.
    #[must_use]
    pub const fn with_capture_response(mut self, capture_response: bool) -> Self {
        self.capture_response = capture_response;
        self
    }
}

impl Default for HttpTraceConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds response middleware that records an HTTP trace segment through a sink.
///
/// The middleware creates an exporter-neutral [`TraceSegment`] after route
/// handling. Callers can render the segment as X-Ray, convert it to
/// OpenTelemetry, store it for tests, or send it to another sink.
pub fn http_trace_response_middleware(
    tracer: Tracer,
    sink: Arc<HttpTraceSink>,
) -> impl Fn(&Request, Response) -> Response + Clone + Send + Sync + 'static {
    http_trace_response_middleware_with_config(tracer, HttpTraceConfig::new(), sink)
}

/// Builds response middleware that records an HTTP trace segment through a sink.
///
/// This variant accepts explicit response-capture configuration.
pub fn http_trace_response_middleware_with_config(
    tracer: Tracer,
    config: HttpTraceConfig,
    sink: Arc<HttpTraceSink>,
) -> impl Fn(&Request, Response) -> Response + Clone + Send + Sync + 'static {
    move |request, response| {
        let segment = record_http_trace(&tracer, request, &response, config);
        sink(segment);
        response
    }
}

/// Records an exporter-neutral trace segment for an HTTP response.
///
/// The segment name uses the matched route label, or `NOT_FOUND` for unmatched
/// requests. Method, path, route, and status are recorded as metadata. Cold
/// start and service name are recorded as annotations.
#[must_use]
pub fn record_http_trace(
    tracer: &Tracer,
    request: &Request,
    response: &Response,
    config: HttpTraceConfig,
) -> TraceSegment {
    let route = request
        .matched_route()
        .map_or_else(|| NOT_FOUND_ROUTE.to_owned(), crate::MatchedRoute::label);
    let mut segment = tracer.segment(route.clone());

    segment.add_annotation("ColdStart", cold_start::is_cold_start());
    segment.add_annotation("Service", tracer.service_name());
    segment.add_metadata("httpMethod", request.method().as_str());
    segment.add_metadata("path", request.path());
    segment.add_metadata("route", route);
    segment.add_metadata("statusCode", u64::from(response.status_code()));

    if config.capture_response() && tracer.captures_response() && is_json_response(response) {
        if let Some(response_value) = response_json_trace_value(response) {
            segment.capture_response(response_value);
        }
    }

    if tracer.captures_error() && response.status_code() >= 500 {
        segment.capture_error(format!("HTTP {}", response.status_code()));
    }

    segment
}

fn is_json_response(response: &Response) -> bool {
    response.header("content-type").is_some_and(|content_type| {
        let content_type = content_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();

        content_type == "application/json" || content_type.ends_with("+json")
    })
}

fn response_json_trace_value(response: &Response) -> Option<TraceValue> {
    serde_json::from_slice(response.body())
        .ok()
        .map(trace_value_from_json)
}

fn trace_value_from_json(value: serde_json::Value) -> TraceValue {
    match value {
        serde_json::Value::Null => TraceValue::null(),
        serde_json::Value::Bool(value) => TraceValue::from(value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                TraceValue::from(value)
            } else if let Some(value) = value.as_u64() {
                TraceValue::from(value)
            } else {
                TraceValue::from(value.as_f64())
            }
        }
        serde_json::Value::String(value) => TraceValue::from(value),
        serde_json::Value::Array(values) => {
            TraceValue::array(values.into_iter().map(trace_value_from_json))
        }
        serde_json::Value::Object(fields) => {
            let mut output = TraceFields::new();
            for (key, value) in fields {
                output.insert(key, trace_value_from_json(value));
            }
            TraceValue::from(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use victors_lambdas_tracer::{TraceValue, Tracer, TracerConfig};

    use super::{
        HttpTraceConfig, http_trace_response_middleware,
        http_trace_response_middleware_with_config, record_http_trace,
    };
    use crate::{Method, Request, Response};

    fn test_tracer() -> Tracer {
        Tracer::with_config(TracerConfig::new("checkout"))
    }

    #[test]
    fn records_http_trace_with_route_metadata_and_json_response() {
        let tracer = test_tracer();
        let mut request = Request::new(Method::Get, "/orders/order-1");
        request.set_matched_route(Method::Get, "/orders/{order_id}");
        let response = Response::ok(br#"{"ok":true}"#.to_vec())
            .with_header("Content-Type", "application/json");

        let segment = record_http_trace(&tracer, &request, &response, HttpTraceConfig::new());

        assert_eq!(segment.name(), "GET /orders/{order_id}");
        assert_eq!(
            segment.annotations().get("Service"),
            Some(&TraceValue::from("checkout"))
        );
        assert!(segment.annotations().contains_key("ColdStart"));
        assert_eq!(
            segment.metadata().get("httpMethod"),
            Some(&TraceValue::from("GET"))
        );
        assert_eq!(
            segment.metadata().get("path"),
            Some(&TraceValue::from("/orders/order-1"))
        );
        assert_eq!(
            segment.metadata().get("route"),
            Some(&TraceValue::from("GET /orders/{order_id}"))
        );
        assert_eq!(
            segment.metadata().get("statusCode"),
            Some(&TraceValue::from(200_u64))
        );
        assert_eq!(
            segment
                .response()
                .map(TraceValue::to_json_string)
                .as_deref(),
            Some(r#"{"ok":true}"#)
        );
        assert_eq!(segment.error(), None);
    }

    #[test]
    fn records_not_found_and_http_fault_trace() {
        let tracer = test_tracer();
        let request = Request::new(Method::Post, "/missing");
        let response = Response::new(503);

        let segment = record_http_trace(&tracer, &request, &response, HttpTraceConfig::new());

        assert_eq!(segment.name(), "NOT_FOUND");
        assert_eq!(
            segment.metadata().get("route"),
            Some(&TraceValue::from("NOT_FOUND"))
        );
        assert_eq!(segment.error(), Some(&TraceValue::from("HTTP 503")));
    }

    #[test]
    fn trace_config_can_disable_response_capture() {
        let tracer = test_tracer();
        let request = Request::new(Method::Get, "/orders");
        let response = Response::ok(br#"{"ok":true}"#.to_vec())
            .with_header("Content-Type", "application/json");

        let segment = record_http_trace(
            &tracer,
            &request,
            &response,
            HttpTraceConfig::new().with_capture_response(false),
        );

        assert_eq!(segment.response(), None);
    }

    #[test]
    fn response_middleware_sends_completed_segments_to_sink() {
        let tracer = test_tracer();
        let segments = Arc::new(Mutex::new(Vec::new()));
        let sink_segments = Arc::clone(&segments);
        let sink = Arc::new(move |segment| {
            sink_segments.lock().expect("segment sink").push(segment);
        });
        let request = Request::new(Method::Get, "/orders");

        let response = http_trace_response_middleware(tracer, sink)(&request, Response::ok("ok"));

        assert_eq!(response.status_code(), 200);
        assert_eq!(segments.lock().expect("segments").len(), 1);
    }

    #[test]
    fn configured_response_middleware_sends_segments_to_sink() {
        let tracer = test_tracer();
        let segments = Arc::new(Mutex::new(Vec::new()));
        let sink_segments = Arc::clone(&segments);
        let sink = Arc::new(move |segment| {
            sink_segments.lock().expect("segment sink").push(segment);
        });
        let request = Request::new(Method::Get, "/orders");

        let response = http_trace_response_middleware_with_config(
            tracer,
            HttpTraceConfig::new().with_capture_response(false),
            sink,
        )(&request, Response::ok("ok"));

        assert_eq!(response.status_code(), 200);
        assert_eq!(segments.lock().expect("segments").len(), 1);
    }
}
