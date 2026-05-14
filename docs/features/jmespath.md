# JMESPath

The JMESPath utility extracts data from JSON-like Lambda events using JMESPath expressions. It is exposed through the
`jmespath` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["jmespath"] }
```

## Supported Behavior

- `search` for one-off JMESPath extraction into `serde_json::Value`.
- `JmespathExpression` for compiling and reusing expressions.
- `search_as` and `extract_data_from_envelope` for decoding selected data into Rust types.
- Powertools decode functions available in every expression: `powertools_json`, `powertools_base64`, and
  `powertools_base64_gzip`.
- Built-in envelope constants for common Lambda event extraction paths such as API Gateway body JSON, SQS message body
  JSON, SNS message JSON, EventBridge detail, Kinesis base64 JSON records, CloudWatch Logs base64-gzip payloads, and S3
  notifications delivered through SQS or Kinesis Firehose.
- Optional idempotency key extraction via `idempotency-jmespath`.

## Snippet

The buildable snippet in [examples/snippets/jmespath/src/main.rs](../../examples/snippets/jmespath/src/main.rs)
extracts an API Gateway body with `powertools_json(body)` and performs a direct request-context lookup.

Run it locally with:

```sh
cargo run -p jmespath-snippet
```
