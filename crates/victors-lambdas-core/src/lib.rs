//! Shared foundations for Victor's Lambdas crates.

pub mod cold_start;
pub mod config;
pub mod env;
pub mod metadata;

pub use config::{ServiceConfig, ServiceConfigBuilder};
