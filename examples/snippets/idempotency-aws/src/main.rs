//! AWS-backed idempotency snippet for documentation.

use std::{env, error::Error, time::Duration};

use aws_config::BehaviorVersion;
use serde::{Deserialize, Serialize};
use victors_lambdas::prelude::{
    AsyncIdempotency, CachedIdempotencyStore, DynamoDbIdempotencyStore, IdempotencyConfig,
    key_from_json_pointer,
};

#[derive(Serialize)]
struct CheckoutRequest {
    request_id: String,
    customer_id: String,
    amount_cents: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct CheckoutResponse {
    confirmation_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if env::var_os("RUN_AWS_IDEMPOTENCY_SNIPPET").is_none() {
        println!("set RUN_AWS_IDEMPOTENCY_SNIPPET=1 to run AWS idempotency calls");
        return Ok(());
    }

    let table_name = env::var("IDEMPOTENCY_TABLE")?;
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let store = CachedIdempotencyStore::new(
        DynamoDbIdempotencyStore::new(aws_sdk_dynamodb::Client::new(&config), table_name)
            .with_key_attr("id")
            .with_expiry_attr("expiration")
            .with_status_attr("status")
            .with_data_attr("data")
            .with_validation_attr("validation"),
    );
    let idempotency = AsyncIdempotency::with_config(
        store,
        IdempotencyConfig::from_env()
            .with_key_prefix("checkout")
            .with_lambda_remaining_time(Duration::from_secs(25)),
    );

    let request = CheckoutRequest {
        request_id: "request-123".to_owned(),
        customer_id: "customer-456".to_owned(),
        amount_cents: 4_299,
    };
    let payload = serde_json::to_value(&request)?;
    let key = key_from_json_pointer(&payload, "/request_id")?;

    let outcome = idempotency
        .execute_json_with_key(key, &payload, || async {
            Ok::<_, std::io::Error>(CheckoutResponse {
                confirmation_id: "confirmation-789".to_owned(),
            })
        })
        .await?;

    println!("{}", serde_json::to_string(outcome.value())?);

    Ok(())
}
