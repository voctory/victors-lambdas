//! Idempotency utility.

mod config;
mod error;
mod key;
mod payload;
mod record;
mod status;
mod store;
mod workflow;

pub use config::IdempotencyConfig;
pub use error::{IdempotencyError, IdempotencyExecutionError, IdempotencyResult};
pub use key::IdempotencyKey;
pub use payload::{hash_payload, key_from_json_pointer, key_from_payload};
pub use record::IdempotencyRecord;
pub use status::IdempotencyStatus;
pub use store::{
    AsyncIdempotencyStore, IdempotencyStore, IdempotencyStoreError, IdempotencyStoreFuture,
    IdempotencyStoreResult, InMemoryIdempotencyStore,
};
pub use workflow::{AsyncIdempotency, Idempotency, IdempotencyOutcome};
