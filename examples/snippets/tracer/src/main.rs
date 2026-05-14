//! Tracer snippet for documentation.

use std::error::Error;
use std::time::Duration;

use aws_lambda_powertools::prelude::{
    TraceSegment, TraceValue, Tracer as PowertoolsTracer, TracerConfig, XrayDaemonClient,
    XrayDaemonConfig,
};
use opentelemetry::{
    Context,
    trace::{Span as _, Tracer as _, TracerProvider as _},
};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};

fn main() -> Result<(), Box<dyn Error>> {
    let tracer = PowertoolsTracer::with_config(TracerConfig::new("checkout"));
    let context = tracer.context_from_xray_header(
        "handler",
        "Root=1-67891233-abcdef012345678912345678;Parent=53995c3f42cd8ad8;Sampled=1",
    );

    let segment = tracer
        .segment_with_context(context)
        .with_annotation("tenant", "north")
        .with_metadata("order_id", "order-123")
        .with_response(TraceValue::object([("status", "accepted")]));
    let document = segment.to_xray_subsegment_document(
        "70de5b6f19ff9a0a",
        1_700_000_000.0,
        1_700_000_000.25,
    )?;

    println!("{document}");

    let otel_provider = stdout_tracer_provider("checkout");
    export_segment(&otel_provider, &segment);
    otel_provider.shutdown()?;

    if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT") {
        let otlp_provider = otlp_http_tracer_provider("checkout", &endpoint)?;
        export_segment(&otlp_provider, &segment);
        otlp_provider.shutdown()?;
    }

    let daemon = XrayDaemonClient::new(XrayDaemonConfig::new("127.0.0.1:2000"));
    assert_eq!(daemon.address(), "127.0.0.1:2000");

    Ok(())
}

fn export_segment(provider: &SdkTracerProvider, segment: &TraceSegment) {
    let otel_tracer = provider.tracer("powertools-lambda-rust");
    let mut otel_span =
        otel_tracer.build_with_context(segment.to_otel_span_builder(), &Context::current());
    otel_span.end();
}

fn stdout_tracer_provider(service_name: &'static str) -> SdkTracerProvider {
    let exporter = opentelemetry_stdout::SpanExporter::default();
    SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_resource(Resource::builder().with_service_name(service_name).build())
        .build()
}

fn otlp_http_tracer_provider(
    service_name: &'static str,
    endpoint: &str,
) -> Result<SdkTracerProvider, opentelemetry_otlp::ExporterBuildError> {
    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_secs(3))
        .with_protocol(Protocol::HttpBinary)
        .build()?;

    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_service_name(service_name).build())
        .build())
}
