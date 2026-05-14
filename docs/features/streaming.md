# Streaming

The streaming utility provides a seekable reader over byte-range sources, including S3 object range sources. It is
exposed through the `streaming` Cargo feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["streaming"] }
```

Use `streaming-s3` to enable the AWS SDK-backed S3 adapter:

```toml
aws-lambda-powertools = { version = "0.1", features = ["streaming-s3"] }
```

## Supported Behavior

- `SeekableStream` implements `Read` and `Seek` over a caller-provided `RangeSource`.
- `S3RangeSource` models S3 object range reads through the `S3ObjectClient` trait.
- Optional `streaming-s3` exposes `AwsSdkS3ObjectClient` for a configured `aws_sdk_s3::Client`.
- Seeking invalidates the active range only when the stream position changes.
- `BytesRangeSource` supports local buffers for examples and tests.
- Optional `streaming-gzip` exposes `gzip_decoder`.
- Optional `streaming-csv` exposes `csv_reader` and `csv_reader_with_builder`.
- Optional `streaming-zip` exposes `zip_archive`.

## Snippet

The buildable snippet in [examples/snippets/streaming/src/main.rs](../../examples/snippets/streaming/src/main.rs)
seeks within a byte-range stream and then parses it as CSV.

Run it locally with:

```sh
cargo run -p streaming-snippet
```
