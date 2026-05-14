//! Logger snippet for documentation.

use victors_lambdas::prelude::{LambdaContextFields, LogLevel, LogValue, Logger, LoggerConfig};

fn main() {
    let context = LambdaContextFields::new("request-1", "checkout-fn")
        .with_function_version("$LATEST")
        .with_function_memory_size(256)
        .with_cold_start(true);

    let mut logger = Logger::with_config(
        LoggerConfig::new("checkout")
            .with_level(LogLevel::Info)
            .with_event_logging(true)
            .with_sample_rate(0.0),
    )
    .with_lambda_context(&context)
    .with_correlation_id("order-123")
    .with_redacted_field("card_number");

    logger.append_field("component", "orders-api");

    let event = LogValue::object([
        ("path", LogValue::from("/orders")),
        ("card_number", LogValue::from("4111111111111111")),
    ]);

    if let Some(line) = logger
        .info("order accepted")
        .field("order_id", "order-123")
        .field("card_number", "4111111111111111")
        .event(event)
        .render()
    {
        println!("{line}");
    }
}
