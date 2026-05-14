//! Idempotency snippet for documentation.

use std::{convert::Infallible, error::Error};

use serde::{Deserialize, Serialize};
use victors_lambdas::prelude::{
    CachedIdempotencyStore, Idempotency, IdempotencyConfig, InMemoryIdempotencyStore,
    key_from_jmespath,
};

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
    let config = IdempotencyConfig::from_env()
        .with_key_prefix("checkout")
        .with_payload_validation_jmespath("powertools_json(body)");
    let store = CachedIdempotencyStore::new(InMemoryIdempotencyStore::new());
    let mut idempotency = Idempotency::with_config(store, config);

    let request = CheckoutRequest {
        request_id: "request-123".to_owned(),
        customer_id: "customer-456".to_owned(),
        amount_cents: 4_299,
    };
    let payload = serde_json::json!({
        "body": serde_json::to_string(&request)?,
        "requestContext": {
            "requestTimeEpoch": 1_779_886_400_i64,
        },
    });
    let retry_payload = serde_json::json!({
        "body": serde_json::to_string(&request)?,
        "requestContext": {
            "requestTimeEpoch": 1_779_886_401_i64,
        },
    });
    let key = key_from_jmespath(&payload, "powertools_json(body).request_id")?;
    let retry_key = key_from_jmespath(&retry_payload, "powertools_json(body).request_id")?;
    assert_eq!(key, retry_key);

    let first = idempotency.execute_json_with_key(key.clone(), &payload, || {
        Ok::<CheckoutResponse, Infallible>(CheckoutResponse {
            confirmation_id: "confirmation-789".to_owned(),
        })
    })?;
    assert!(first.is_executed());

    let second = idempotency.execute_json_with_key(retry_key, &retry_payload, || {
        Ok::<CheckoutResponse, Infallible>(CheckoutResponse {
            confirmation_id: "should-not-run".to_owned(),
        })
    })?;
    assert!(second.is_replayed());
    assert_eq!(idempotency.store().cache_len()?, 1);
    assert_eq!(
        second.value(),
        &CheckoutResponse {
            confirmation_id: "confirmation-789".to_owned(),
        }
    );

    println!("{}", serde_json::to_string(second.value())?);

    Ok(())
}
