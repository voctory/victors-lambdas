# Lambda Metadata

The metadata utility reads the Lambda execution-environment metadata endpoint exposed through
`AWS_LAMBDA_METADATA_API` and `AWS_LAMBDA_METADATA_TOKEN`.

When the function is not running in Lambda, or when `POWERTOOLS_DEV` is enabled, the utility returns empty metadata
without making a network request. Successful endpoint responses are cached for the process lifetime.

## Install

```toml
victors-lambdas = { version = "0.1", features = ["metadata"] }
```

## Usage

```rust
use victors_lambdas::metadata::get_lambda_metadata;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metadata = get_lambda_metadata()?;

    if let Some(availability_zone_id) = metadata.availability_zone_id() {
        println!("availability_zone_id={availability_zone_id}");
    }

    Ok(())
}
```

See `examples/snippets/metadata` for a buildable version of this snippet.
