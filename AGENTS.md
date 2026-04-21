# Repository Guidelines

This repository is an unofficial Rust toolkit for AWS Lambda functions. Keep language precise: do not imply this is an
official AWS-owned project unless that becomes explicitly true. Keep personal ownership details limited to copyright
notices and licensing files.

## Collaboration and Git Safety

- Make small, frequent, atomic commits. Each commit should be independently reviewable and explain one change.
- Before committing, inspect staged changes with `git diff --cached --stat` and stage with explicit pathspecs.
- Do not run destructive git operations such as `git reset --hard`, `git clean -fd`, force checkouts, or force pushes
  unless explicitly requested.
- Assume other humans or agents may have local changes. Do not revert work you did not create.
- Keep generated output, local logs, and scratch files out of commits unless they are source-of-truth fixtures.

## Reference Repositories

- Public Powertools repositories can be used for API behavior, docs organization, feature inventory, and compatibility
  planning.
- Proprietary adjacent repositories may be consulted only for generic Rust workspace practices. Do not copy code,
  architecture, names, business logic, comments, or implementation details from proprietary repositories.
- If a behavior is copied from an upstream public project, prefer documenting the compatibility goal rather than copying
  implementation text.

## Rust Workspace Shape

- Keep the root as a virtual Cargo workspace with `resolver = "3"`.
- Put reusable crates under `crates/<crate-name>/`.
- Put runnable workspace examples under `examples/<example-name>/`.
- Keep one primary umbrella crate, `aws-lambda-powertools`, and expose optional utilities through Cargo features.
- Keep shared foundations in `aws-lambda-powertools-core`, but avoid turning it into a dumping ground.
- Commit `Cargo.lock` for reproducible workspace checks and examples.
- Keep `rust-toolchain.toml` aligned with `workspace.package.rust-version`, and document any MSRV change.

## Rust Design Rules

- Keep `lib.rs` thin: module declarations, re-exports, and crate-level docs only.
- Split non-trivial behavior into ownership-named modules such as `config`, `env`, `logger`, `metric`, `provider`,
  `record`, `route`, `router`, `validation`, or `context`.
- Avoid catch-all modules and files named `utils`, `helpers`, `misc`, `shared`, or `common` unless there is a precise,
  documented ownership boundary.
- Model finite states as enums.
- Separate raw event/provider DTOs from parsed domain inputs once real AWS integrations are added.
- Keep constructors private or validating when a type carries invariants.
- Use Cargo features for optional AWS/provider integrations so Lambda users do not pay for dependencies they do not use.
- Prefer zero hidden global state unless Lambda execution-environment behavior requires it, such as cold-start tracking.
- Keep public APIs documented enough for `missing_docs = "warn"` and docs.rs.

## Validation Commands

Run the smallest relevant command while iterating, then run the full suite before handoff:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```

Use the Lambda release profile only for packaging/performance validation:

```sh
cargo build --profile release-lambda --workspace
```

## Testing Expectations

- Put unit tests near the module they exercise.
- Put integration tests under each crate's `tests/` directory when the test uses the public crate surface.
- Put shared event fixtures under `tests/events/` once parser/event work starts.
- Gate AWS integration and end-to-end tests behind explicit environment variables so normal local and CI checks remain
  deterministic.
- Add snippet examples that compile in CI before using them in docs.

## Documentation Expectations

- Keep planning and contributor docs in `docs/`.
- Keep API documentation in rustdoc comments close to public items.
- Keep examples short and buildable; docs should link to examples instead of duplicating large code blocks.
- When adding a feature, update the porting plan or replace its checklist item with a more specific design document.
