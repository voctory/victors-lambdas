//! Validation snippet for documentation.

use std::error::Error;

use serde_json::json;
use victors_lambdas::prelude::{JsonSchemaCache, Validate, ValidationResult, Validator};

#[derive(Debug, Eq, PartialEq)]
struct CreateOrder {
    order_id: String,
    quantity: i64,
}

impl Validate for CreateOrder {
    fn validate(&self, validator: &Validator) -> ValidationResult {
        validator.required_text_field("order_id", &self.order_id)?;
        validator.text_min_chars("order_id", &self.order_id, 3)?;
        validator.i64_in_range("quantity", self.quantity, 1, 10)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let validator = Validator::new();
    let order = validator.validate_inbound(CreateOrder {
        order_id: "order-123".to_owned(),
        quantity: 2,
    })?;

    let schema = json!({
        "type": "object",
        "required": ["order_id", "quantity"],
        "properties": {
            "order_id": { "type": "string" },
            "quantity": { "type": "integer", "minimum": 1 }
        }
    });
    let instance = json!({
        "order_id": order.order_id,
        "quantity": order.quantity
    });
    let event = json!({
        "body": serde_json::to_string(&instance)?,
        "requestContext": {
            "requestId": "ignored",
        },
    });
    let mut schemas = JsonSchemaCache::new();
    schemas.validate_or_compile_envelope(
        "create-order",
        &schema,
        &event,
        "powertools_json(body)",
    )?;

    println!("accepted {}", instance["order_id"]);

    Ok(())
}
