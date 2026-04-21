//! Idempotency utility.

mod config;
mod key;
mod record;
mod status;
mod store;

pub use config::IdempotencyConfig;
pub use key::IdempotencyKey;
pub use record::IdempotencyRecord;
pub use status::IdempotencyStatus;
pub use store::{
    IdempotencyStore, IdempotencyStoreError, IdempotencyStoreResult, InMemoryIdempotencyStore,
};
