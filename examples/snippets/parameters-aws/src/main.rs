//! AWS-backed parameters snippet for documentation.

use std::{env, error::Error, time::Duration};

use aws_config::BehaviorVersion;
use aws_lambda_powertools::prelude::{
    AppConfigProvider, AsyncParameters, CachePolicy, DynamoDbParameterProvider, ParameterTransform,
    SecretsManagerProvider, SsmParameterProvider, SsmParameterType,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct FeatureConfig {
    checkout_enabled: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if env::var_os("RUN_AWS_PARAMETERS_SNIPPET").is_none() {
        println!("set RUN_AWS_PARAMETERS_SNIPPET=1 to run AWS provider calls");
        return Ok(());
    }

    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;

    let ssm_provider =
        SsmParameterProvider::new(aws_sdk_ssm::Client::new(&config)).with_decryption(true);
    let ssm_parameters = AsyncParameters::with_cache_policy(
        ssm_provider.clone(),
        CachePolicy::ttl(Duration::from_secs(60)),
    );
    if let Some(database) = ssm_parameters
        .get_json::<DatabaseConfig>("/checkout/database.json")
        .await?
    {
        println!("database endpoint: {}:{}", database.host, database.port);
    }

    let by_name = ssm_provider
        .get_parameters_by_name(["/checkout/service-name", "/checkout/log-level"])
        .await?;
    println!(
        "loaded {} SSM parameters by name; invalid names: {:?}",
        by_name.parameters().len(),
        by_name.invalid_names()
    );

    let path_parameters = ssm_provider
        .get_parameters_by_path_recursive("/checkout")
        .await?;
    println!("loaded {} SSM path parameters", path_parameters.len());

    if let Ok(value) = env::var("CHECKOUT_PARAMETER_VALUE") {
        let version = ssm_provider
            .set_parameter_with_options(
                "/checkout/last-deploy",
                &value,
                SsmParameterType::String,
                true,
            )
            .await?;
        println!("updated /checkout/last-deploy to version {version}");
    }

    let secrets = AsyncParameters::new(SecretsManagerProvider::new(
        aws_sdk_secretsmanager::Client::new(&config),
    ));
    if let Some(secret) = secrets
        .get_transformed("checkout/api-key", ParameterTransform::None)
        .await?
    {
        println!(
            "loaded secret with {} bytes",
            secret.as_text().unwrap_or_default().len()
        );
    }

    let appconfig = AsyncParameters::new(AppConfigProvider::new(
        aws_sdk_appconfigdata::Client::new(&config),
        "checkout",
        "prod",
    ));
    if let Some(flags) = appconfig
        .get_json::<FeatureConfig>("checkout-flags")
        .await?
    {
        println!("checkout enabled: {}", flags.checkout_enabled);
    }

    let dynamodb_provider = DynamoDbParameterProvider::new(
        aws_sdk_dynamodb::Client::new(&config),
        "checkout-parameters",
    )
    .with_key_attr("pk")
    .with_sort_attr("sk")
    .with_value_attr("value");
    let dynamodb_parameters = AsyncParameters::with_cache_policy(
        dynamodb_provider.clone(),
        CachePolicy::ttl(Duration::from_secs(60)),
    );
    if let Some(parameter) = dynamodb_parameters.get("/checkout/feature").await? {
        println!("loaded DynamoDB parameter {}", parameter.name());
    }

    let table_parameters = dynamodb_provider
        .get_parameters_by_path("/checkout")
        .await?;
    println!("loaded {} DynamoDB path parameters", table_parameters.len());

    Ok(())
}
