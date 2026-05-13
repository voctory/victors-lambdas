//! Parameters snippet for documentation.

use std::{error::Error, time::Duration};

use aws_lambda_powertools::prelude::{
    CachePolicy, InMemoryParameterProvider, ParameterTransform, Parameters,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct DatabaseConfig {
    host: String,
    port: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    let provider = InMemoryParameterProvider::new()
        .with_parameter("/checkout/service_name", "checkout")
        .with_parameter(
            "/checkout/database.json",
            r#"{"host":"db.internal","port":5432}"#,
        )
        .with_parameter("/checkout/token.binary", "c2VjcmV0LXRva2Vu");

    let parameters =
        Parameters::with_cache_policy(provider, CachePolicy::ttl(Duration::from_secs(60)));

    let service = parameters
        .get("/checkout/service_name")
        .expect("service name parameter should exist");
    assert_eq!(service.value(), "checkout");

    let database = parameters
        .get_json::<DatabaseConfig>("/checkout/database.json")?
        .expect("database config parameter should exist");
    assert_eq!(
        database,
        DatabaseConfig {
            host: "db.internal".to_owned(),
            port: 5432,
        }
    );

    let token = parameters
        .get_transformed("/checkout/token.binary", ParameterTransform::Auto)?
        .expect("token parameter should exist");
    assert_eq!(token.as_binary(), Some(&b"secret-token"[..]));

    println!("{}:{} as {}", database.host, database.port, service.value());

    Ok(())
}
