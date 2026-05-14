//! Idempotency utility.

mod config;
#[cfg(feature = "dynamodb")]
mod dynamodb;
mod error;
mod key;
mod payload;
mod record;
mod status;
mod store;
mod workflow;

pub use config::{IdempotencyConfig, PayloadValidation};
#[cfg(feature = "dynamodb")]
pub use dynamodb::DynamoDbIdempotencyStore;
pub use error::{IdempotencyError, IdempotencyExecutionError, IdempotencyResult};
pub use key::IdempotencyKey;
pub use payload::{hash_payload, key_from_json_pointer, key_from_payload};
#[cfg(feature = "jmespath")]
pub use payload::{hash_payload_from_jmespath, key_from_jmespath};
pub use record::IdempotencyRecord;
pub use status::IdempotencyStatus;
pub use store::{
    AsyncIdempotencyStore, CachedIdempotencyStore, IdempotencyStore, IdempotencyStoreError,
    IdempotencyStoreFuture, IdempotencyStoreResult, InMemoryIdempotencyStore,
};
pub use workflow::{AsyncIdempotency, Idempotency, IdempotencyOutcome};
