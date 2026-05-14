# Release Checklist

Victor's Lambdas uses a synchronized workspace version. Publish support crates
before the umbrella crate so first-time internal dependencies resolve from
crates.io.

## Pre-Publish Gates

Run from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo check --workspace --all-targets --all-features --locked
cargo doc --workspace --all-features --no-deps
cargo build --profile release-lambda --workspace
```

Run a dry-run for each crate before the matching publish command:

```sh
cargo publish --dry-run -p <crate-name>
```

For the first crates.io release, publish in this order:

1. `victors-lambdas-core`
2. `victors-lambdas-data-masking`
3. `victors-lambdas-jmespath`
4. `victors-lambdas-kafka`
5. `victors-lambdas-parameters`
6. `victors-lambdas-streaming`
7. `victors-lambdas-logger`
8. `victors-lambdas-metadata`
9. `victors-lambdas-metrics`
10. `victors-lambdas-tracer`
11. `victors-lambdas-validation`
12. `victors-lambdas-idempotency`
13. `victors-lambdas-feature-flags`
14. `victors-lambdas-testing`
15. `victors-lambdas-parser`
16. `victors-lambdas-batch`
17. `victors-lambdas-event-handler`
18. `victors-lambdas`

Re-run the dry-run for each dependent crate after its internal dependencies are
available in the registry.

## Package Review

Before publishing a crate, inspect the files Cargo will upload:

```sh
cargo package --list -p <crate-name>
```

Check generated crate archive sizes after dry-runs:

```sh
ls -lh target/package/*.crate
```

crates.io currently rejects crate archives above 10 MB. Keep docs, generated
artifacts, local logs, and scratch files out of published packages unless they
are required source-of-truth fixtures.

## docs.rs Coverage

The local docs gate is:

```sh
cargo doc --workspace --all-features --no-deps
```

Each published crate inherits license, repository, homepage, README, keywords,
categories, Rust edition, and MSRV from the workspace. The README uses absolute
GitHub links for documentation paths so crates.io-rendered pages do not point
at files outside each crate archive.

## Provenance And SBOM Baseline

Before publishing, record the source revision and package checksums under the
ignored `target/package/` directory:

```sh
git status --short
git rev-parse HEAD
cargo metadata --locked --all-features --format-version 1 > target/package/cargo-metadata.json
shasum -a 256 target/package/*.crate > target/package/SHA256SUMS
```

The Cargo metadata JSON is the dependency graph baseline for the release. Add a
formal CycloneDX or SPDX SBOM generator later only if downstream users need that
artifact; do not block the `0.1.0` pre-release on a tool-specific SBOM format.

## Publish

After `cargo login`, publish each crate in the order above:

```sh
cargo publish -p <crate-name>
```

After all crates are visible on crates.io, tag the exact published commit:

```sh
git tag -a v0.1.0 -m "Release 0.1.0"
git push origin v0.1.0
```
