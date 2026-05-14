# Streaming

The streaming utility provides a seekable reader over byte-range sources. It is exposed through the `streaming` Cargo
feature on the umbrella crate:

```toml
aws-lambda-powertools = { version = "0.1", features = ["streaming"] }
```

## Supported Behavior

- `SeekableStream` implements `Read` and `Seek` over a caller-provided `RangeSource`.
- Seeking invalidates the active range only when the stream position changes.
- `BytesRangeSource` supports local buffers for examples and tests.
- Optional `streaming-gzip` exposes `gzip_decoder`.
- Optional `streaming-csv` exposes `csv_reader` and `csv_reader_with_builder`.
- Optional `streaming-zip` exposes `zip_archive`.

An AWS SDK-backed S3 range source is not implemented yet.

## Snippet

The buildable snippet in [examples/snippets/streaming/src/main.rs](../../examples/snippets/streaming/src/main.rs)
seeks within a byte-range stream and then parses it as CSV.

Run it locally with:

```sh
cargo run -p streaming-snippet
```
