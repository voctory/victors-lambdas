//! Buildable `JMESPath` utility snippet.

use serde::Deserialize;
use serde_json::json;
use victors_lambdas::jmespath::{API_GATEWAY_REST, extract_data_from_envelope, search};

#[derive(Debug, Deserialize)]
struct Order {
    order_id: String,
    quantity: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event = json!({
        "body": "{\"order_id\":\"order-1\",\"quantity\":2}",
        "requestContext": {
            "requestId": "request-1"
        }
    });

    let order: Order = extract_data_from_envelope(event.clone(), API_GATEWAY_REST)?;
    let request_id = search("requestContext.requestId", event)?;

    println!(
        "order={} quantity={} request_id={}",
        order.order_id, order.quantity, request_id
    );

    Ok(())
}
