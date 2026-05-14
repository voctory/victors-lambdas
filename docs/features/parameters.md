# Parameters

The parameters utility retrieves configuration values through a provider facade with optional caching and transforms. It
is exposed through the `parameters` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["parameters"] }
```

## Providers

`InMemoryParameterProvider` is useful for tests, examples, and local workflows. Optional AWS-backed providers are
available behind dedicated umbrella features:

| Feature | Provider |
| --- | --- |
| `parameters-ssm` | `SsmParameterProvider` |
| `parameters-secrets` | `SecretsManagerProvider` |
| `parameters-appconfig` | `AppConfigProvider` |
| `parameters-dynamodb` | `DynamoDbParameterProvider` |

AWS-backed providers accept SDK clients from the caller so Lambda handlers can choose how they load AWS configuration
and so the base parameters feature does not force AWS SDK dependencies on every user.

## Supported Behavior

- Sync and async provider traits.
- Parameter facade caching with disabled, forever, and TTL policies.
- Force-fetch helpers that refresh or remove cached values.
- Text, JSON, binary, and suffix-based auto transforms.
- In-memory provider for tests and examples.
- Optional SSM, Secrets Manager, AppConfig, and DynamoDB providers.

## Snippet

The buildable snippet in [examples/snippets/parameters/src/main.rs](../../examples/snippets/parameters/src/main.rs)
uses an in-memory provider, TTL caching, JSON deserialization, and automatic binary decoding based on a `.binary`
parameter suffix.

Run it locally with:

```sh
cargo run -p parameters-snippet
```

Use `get_force`, `get_force_json`, `get_force_binary`, or `get_force_transformed` when a Lambda invocation must bypass
the cache and refresh the stored value.

## AWS Provider Snippet

The buildable AWS-backed snippet in
[examples/snippets/parameters-aws/src/main.rs](../../examples/snippets/parameters-aws/src/main.rs) shows how to create
SSM Parameter Store, Secrets Manager, AppConfig, and DynamoDB providers from caller-owned AWS SDK clients. It also
demonstrates SSM by-name, path, and write operations.

The snippet is guarded so local validation does not call AWS by default:

```sh
cargo run -p parameters-aws-snippet
RUN_AWS_PARAMETERS_SNIPPET=1 cargo run -p parameters-aws-snippet
```

Set `RUN_AWS_PARAMETERS_SNIPPET=1` only in an environment with AWS credentials, region configuration, and matching
sample resources.
