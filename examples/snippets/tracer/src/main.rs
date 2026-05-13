//! Tracer snippet for documentation.

use std::error::Error;

use aws_lambda_powertools::prelude::{TraceValue, Tracer, TracerConfig};

fn main() -> Result<(), Box<dyn Error>> {
    let tracer = Tracer::with_config(TracerConfig::new("checkout"));
    let context = tracer.context_from_xray_header(
        "handler",
        "Root=1-67891233-abcdef012345678912345678;Parent=53995c3f42cd8ad8;Sampled=1",
    );

    let document = tracer
        .segment_with_context(context)
        .with_annotation("tenant", "north")
        .with_metadata("order_id", "order-123")
        .with_response(TraceValue::object([("status", "accepted")]))
        .to_xray_subsegment_document("70de5b6f19ff9a0a", 1_700_000_000.0, 1_700_000_000.25)?;

    println!("{document}");

    Ok(())
}
