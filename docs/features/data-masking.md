# Data Masking

The data masking utility erases sensitive values in JSON payloads before they are logged, returned, or passed to other
systems. It is exposed through the `data-masking` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["data-masking"] }
```

Use `data-masking-kms` to enable the direct AWS KMS provider:

```toml
aws-lambda-powertools = { version = "0.1", features = ["data-masking-kms"] }
```

## Supported Behavior

- Replace a whole `serde_json::Value` with the default `*****` mask.
- Replace selected fields by JSON Pointer paths such as `/customer/password`.
- Replace selected fields by dot paths such as `customer.password` or `items.0.card`.
- Use fixed, dynamic, custom, or regex masking strategies.
- Encrypt and decrypt JSON payloads through a `DataMaskingProvider`.
- Optional `data-masking-kms` exposes `KmsDataMaskingProvider` for direct AWS KMS encrypt/decrypt calls.
- Configure whether missing field paths return errors or are ignored.

AWS Encryption SDK envelope encryption/cached data keys and full JSONPath update expressions are not implemented yet.

## Snippet

The buildable snippet in [examples/snippets/data-masking/src/main.rs](../../examples/snippets/data-masking/src/main.rs)
masks password and card fields in a JSON payload.

Run it locally with:

```sh
cargo run -p data-masking-snippet
```
