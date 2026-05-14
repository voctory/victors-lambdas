//! Buildable Kafka utility snippet.

use aws_lambda_events::event::kafka::KafkaEvent;
use aws_lambda_powertools::kafka::{KafkaConsumer, KafkaConsumerConfig};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct Order {
    order_id: String,
    quantity: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event: KafkaEvent = serde_json::from_value(json!({
        "eventSource": "aws:kafka",
        "eventSourceArn": "arn:aws:kafka:us-east-1:123456789012:cluster/orders",
        "bootstrapServers": "b-1.example:9098",
        "records": {
            "orders-0": [{
                "topic": "orders",
                "partition": 0,
                "offset": 15,
                "timestamp": 1_690_900_000_000_i64,
                "timestampType": "CREATE_TIME",
                "key": STANDARD.encode("customer-1"),
                "value": STANDARD.encode(r#"{"order_id":"order-1","quantity":2}"#),
                "headers": [{"traceparent": [116, 114, 97, 99, 101]}]
            }]
        }
    }))?;

    let records =
        KafkaConsumer::<String, Order>::new(KafkaConsumerConfig::json_values()).records(event)?;

    for record in records {
        let key = record.key.unwrap_or_default();
        let order = record.value.expect("snippet event has an order value");
        println!(
            "key={} order={} quantity={}",
            key, order.order_id, order.quantity
        );
    }

    Ok(())
}
