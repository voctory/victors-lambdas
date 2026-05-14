//! Handler invocation test harnesses.

use std::{future::Future, path::Path};

use serde::de::DeserializeOwned;

use crate::{FixtureError, LambdaContextStub, load_json_fixture};

/// Test harness for invoking handler-shaped functions with a reusable context.
///
/// The harness intentionally avoids a dependency on `lambda_runtime` so tests can
/// use it with plain Rust functions, typed events, and custom context doubles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerHarness<C = LambdaContextStub> {
    context: C,
}

impl<C> HandlerHarness<C> {
    /// Creates a handler harness with the supplied context.
    #[must_use]
    pub const fn new(context: C) -> Self {
        Self { context }
    }

    /// Returns the context passed to handler invocations.
    #[must_use]
    pub const fn context(&self) -> &C {
        &self.context
    }

    /// Consumes the harness and returns the stored context.
    #[must_use]
    pub fn into_context(self) -> C {
        self.context
    }

    /// Invokes a synchronous handler with an event and the harness context.
    pub fn invoke<E, R, F>(&self, event: E, handler: F) -> R
    where
        F: FnOnce(E, &C) -> R,
    {
        handler(event, &self.context)
    }

    /// Invokes an asynchronous handler with an event and the harness context.
    pub async fn invoke_async<E, R, F, Fut>(&self, event: E, handler: F) -> R
    where
        F: FnOnce(E, &C) -> Fut,
        Fut: Future<Output = R>,
    {
        handler(event, &self.context).await
    }

    /// Loads a JSON fixture and invokes a synchronous handler with the decoded event.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError`] when the fixture cannot be read or decoded into
    /// the requested event type.
    pub fn invoke_json<E, R, F>(
        &self,
        path: impl AsRef<Path>,
        handler: F,
    ) -> Result<R, FixtureError>
    where
        E: DeserializeOwned,
        F: FnOnce(E, &C) -> R,
    {
        let event = load_json_fixture(path)?;
        Ok(self.invoke(event, handler))
    }

    /// Loads a JSON fixture and invokes an asynchronous handler with the decoded event.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError`] when the fixture cannot be read or decoded into
    /// the requested event type.
    pub async fn invoke_json_async<E, R, F, Fut>(
        &self,
        path: impl AsRef<Path>,
        handler: F,
    ) -> Result<R, FixtureError>
    where
        E: DeserializeOwned,
        F: FnOnce(E, &C) -> Fut,
        Fut: Future<Output = R>,
    {
        let event = load_json_fixture(path)?;
        Ok(self.invoke_async(event, handler).await)
    }
}

impl Default for HandlerHarness {
    fn default() -> Self {
        Self::new(LambdaContextStub::default())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde::Deserialize;

    use super::HandlerHarness;
    use crate::LambdaContextStub;

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct OrderEvent {
        order_id: String,
        quantity: u32,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TenantContext {
        tenant: String,
    }

    #[test]
    fn invokes_sync_handler_with_default_context() {
        let harness = HandlerHarness::default();

        let result = harness.invoke(41, |event, context| {
            format!("{}:{event}", context.request_id())
        });

        assert_eq!(result, "test-request-id:41");
    }

    #[test]
    fn invokes_async_handler_with_custom_context() {
        let harness = HandlerHarness::new(TenantContext {
            tenant: "orders".to_owned(),
        });

        let result = futures_executor::block_on(harness.invoke_async(2, |event, context| {
            let tenant = context.tenant.clone();
            async move { format!("{tenant}:{event}") }
        }));

        assert_eq!(result, "orders:2");
    }

    #[test]
    fn invokes_sync_handler_with_json_fixture() {
        let path = temp_fixture_path("handler-harness-order.json");
        fs::write(&path, r#"{"order_id":"order-1","quantity":2}"#)
            .expect("fixture should be written");

        let harness = HandlerHarness::new(LambdaContextStub::new("request-1", "orders"));
        let result = harness
            .invoke_json(&path, |event: OrderEvent, context| {
                format!(
                    "{}:{}:{}",
                    context.function_name(),
                    event.order_id,
                    event.quantity
                )
            })
            .expect("fixture handler should run");

        assert_eq!(result, "orders:order-1:2");
        fs::remove_file(path).expect("fixture should be removed");
    }

    #[test]
    fn invokes_async_handler_with_json_fixture() {
        let path = temp_fixture_path("handler-harness-async-order.json");
        fs::write(&path, r#"{"order_id":"order-2","quantity":3}"#)
            .expect("fixture should be written");

        let harness = HandlerHarness::default();
        let result = futures_executor::block_on(harness.invoke_json_async(
            &path,
            |event: OrderEvent, context| {
                let request_id = context.request_id().to_owned();
                async move { format!("{request_id}:{}:{}", event.order_id, event.quantity) }
            },
        ))
        .expect("async fixture handler should run");

        assert_eq!(result, "test-request-id:order-2:3");
        fs::remove_file(path).expect("fixture should be removed");
    }

    fn temp_fixture_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("{nanos}-{name}"))
    }
}
