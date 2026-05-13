//! Idempotency snippet for documentation.

use std::{convert::Infallible, error::Error};

use aws_lambda_powertools::prelude::{
    Idempotency, IdempotencyConfig, InMemoryIdempotencyStore, key_from_json_pointer,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct CheckoutRequest {
    request_id: String,
    customer_id: String,
    amount_cents: u64,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CheckoutResponse {
    confirmation_id: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = IdempotencyConfig::from_env().with_key_prefix("checkout");
    let mut idempotency = Idempotency::with_config(InMemoryIdempotencyStore::new(), config);

    let request = CheckoutRequest {
        request_id: "request-123".to_owned(),
        customer_id: "customer-456".to_owned(),
        amount_cents: 4_299,
    };
    let payload = serde_json::to_value(&request)?;
    let key = key_from_json_pointer(&payload, "/request_id")?;

    let first = idempotency.execute_json_with_key(key.clone(), &payload, || {
        Ok::<CheckoutResponse, Infallible>(CheckoutResponse {
            confirmation_id: "confirmation-789".to_owned(),
        })
    })?;
    assert!(first.is_executed());

    let second = idempotency.execute_json_with_key(key, &payload, || {
        Ok::<CheckoutResponse, Infallible>(CheckoutResponse {
            confirmation_id: "should-not-run".to_owned(),
        })
    })?;
    assert!(second.is_replayed());
    assert_eq!(
        second.value(),
        &CheckoutResponse {
            confirmation_id: "confirmation-789".to_owned(),
        }
    );

    println!("{}", serde_json::to_string(second.value())?);

    Ok(())
}
