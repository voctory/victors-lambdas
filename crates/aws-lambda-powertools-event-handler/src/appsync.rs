//! `AppSync` direct resolver routing.

use std::{fmt, future::Future, pin::Pin};

use aws_lambda_events::event::appsync::AppSyncDirectResolverEvent;
use serde_json::Value;

/// `AppSync` direct resolver event using JSON values for event payload fields.
pub type AppSyncEvent = AppSyncDirectResolverEvent<Value, Value, Value>;

/// Handler function for an `AppSync` resolver route.
pub type AppSyncHandler = dyn Fn(&AppSyncEvent) -> Value + Send + Sync + 'static;

/// Boxed future returned by asynchronous `AppSync` resolver handlers.
pub type AppSyncResponseFuture<'a> = Pin<Box<dyn Future<Output = Value> + Send + 'a>>;

/// Asynchronous handler function for an `AppSync` resolver route.
pub type AsyncAppSyncHandler =
    dyn for<'a> Fn(&'a AppSyncEvent) -> AppSyncResponseFuture<'a> + Send + Sync + 'static;

/// Error returned when an `AppSync` event cannot be routed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppSyncResolverError {
    /// No route matched the GraphQL parent type and field name.
    NoRoute {
        /// GraphQL parent type name.
        type_name: String,
        /// GraphQL field name.
        field_name: String,
    },
}

impl fmt::Display for AppSyncResolverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRoute {
                type_name,
                field_name,
            } => {
                write!(
                    formatter,
                    "no AppSync resolver registered for {type_name}.{field_name}"
                )
            }
        }
    }
}

impl std::error::Error for AppSyncResolverError {}

/// Result returned by `AppSync` resolver dispatch.
pub type AppSyncResolverResult<T> = Result<T, AppSyncResolverError>;

/// Registered `AppSync` resolver route.
pub struct AppSyncRoute {
    type_name: String,
    field_name: String,
    handler: Box<AppSyncHandler>,
}

impl AppSyncRoute {
    /// Creates an `AppSync` resolver route.
    #[must_use]
    pub fn new(
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        handler: impl Fn(&AppSyncEvent) -> Value + Send + Sync + 'static,
    ) -> Self {
        Self {
            type_name: type_name.into(),
            field_name: field_name.into(),
            handler: Box::new(handler),
        }
    }

    /// Returns the GraphQL parent type matched by this route.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Returns the GraphQL field matched by this route.
    #[must_use]
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    fn matches(&self, type_name: &str, field_name: &str) -> bool {
        self.field_name == field_name && (self.type_name == type_name || self.type_name == "*")
    }

    fn match_score(&self, type_name: &str, field_name: &str) -> Option<u8> {
        if !self.matches(type_name, field_name) {
            return None;
        }

        if self.type_name == type_name {
            Some(2)
        } else {
            Some(1)
        }
    }

    fn handle(&self, event: &AppSyncEvent) -> Value {
        (self.handler)(event)
    }
}

impl fmt::Debug for AppSyncRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppSyncRoute")
            .field("type_name", &self.type_name)
            .field("field_name", &self.field_name)
            .finish_non_exhaustive()
    }
}

/// Registered asynchronous `AppSync` resolver route.
pub struct AsyncAppSyncRoute {
    type_name: String,
    field_name: String,
    handler: Box<AsyncAppSyncHandler>,
}

impl AsyncAppSyncRoute {
    /// Creates an asynchronous `AppSync` resolver route.
    #[must_use]
    pub fn new(
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        handler: impl for<'a> Fn(&'a AppSyncEvent) -> AppSyncResponseFuture<'a> + Send + Sync + 'static,
    ) -> Self {
        Self {
            type_name: type_name.into(),
            field_name: field_name.into(),
            handler: Box::new(handler),
        }
    }

    /// Returns the GraphQL parent type matched by this route.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Returns the GraphQL field matched by this route.
    #[must_use]
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    fn matches(&self, type_name: &str, field_name: &str) -> bool {
        self.field_name == field_name && (self.type_name == type_name || self.type_name == "*")
    }

    fn match_score(&self, type_name: &str, field_name: &str) -> Option<u8> {
        if !self.matches(type_name, field_name) {
            return None;
        }

        if self.type_name == type_name {
            Some(2)
        } else {
            Some(1)
        }
    }

    fn handle<'a>(&'a self, event: &'a AppSyncEvent) -> AppSyncResponseFuture<'a> {
        (self.handler)(event)
    }
}

impl fmt::Debug for AsyncAppSyncRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AsyncAppSyncRoute")
            .field("type_name", &self.type_name)
            .field("field_name", &self.field_name)
            .finish_non_exhaustive()
    }
}

/// Routes `AppSync` direct Lambda resolver events by GraphQL type and field.
#[derive(Default, Debug)]
pub struct AppSyncResolver {
    routes: Vec<AppSyncRoute>,
}

/// Routes `AppSync` direct Lambda resolver events with async handlers.
#[derive(Default, Debug)]
pub struct AsyncAppSyncResolver {
    routes: Vec<AsyncAppSyncRoute>,
}

impl AppSyncResolver {
    /// Creates an empty `AppSync` resolver.
    #[must_use]
    pub const fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Registers a resolver handler for a GraphQL type and field.
    pub fn resolver(
        &mut self,
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        handler: impl Fn(&AppSyncEvent) -> Value + Send + Sync + 'static,
    ) -> &mut Self {
        self.routes
            .push(AppSyncRoute::new(type_name, field_name, handler));
        self
    }

    /// Registers a resolver handler for a `Query` field.
    pub fn query(
        &mut self,
        field_name: impl Into<String>,
        handler: impl Fn(&AppSyncEvent) -> Value + Send + Sync + 'static,
    ) -> &mut Self {
        self.resolver("Query", field_name, handler)
    }

    /// Registers a resolver handler for a `Mutation` field.
    pub fn mutation(
        &mut self,
        field_name: impl Into<String>,
        handler: impl Fn(&AppSyncEvent) -> Value + Send + Sync + 'static,
    ) -> &mut Self {
        self.resolver("Mutation", field_name, handler)
    }

    /// Registers a resolver handler for any GraphQL parent type with this field.
    pub fn field(
        &mut self,
        field_name: impl Into<String>,
        handler: impl Fn(&AppSyncEvent) -> Value + Send + Sync + 'static,
    ) -> &mut Self {
        self.resolver("*", field_name, handler)
    }

    /// Returns registered resolver routes in insertion order.
    #[must_use]
    pub fn routes(&self) -> &[AppSyncRoute] {
        &self.routes
    }

    /// Dispatches an `AppSync` direct resolver event to a registered route.
    ///
    /// Exact `type.field` routes take precedence over wildcard type routes.
    ///
    /// # Errors
    ///
    /// Returns [`AppSyncResolverError`] when no route is registered for the
    /// event's GraphQL parent type and field.
    pub fn handle(&self, event: &AppSyncEvent) -> AppSyncResolverResult<Value> {
        let route = self.route_for(event)?;
        Ok(route.handle(event))
    }

    /// Dispatches batched `AppSync` direct resolver events in input order.
    ///
    /// # Errors
    ///
    /// Returns [`AppSyncResolverError`] when any event cannot be routed.
    pub fn handle_batch(
        &self,
        events: impl IntoIterator<Item = AppSyncEvent>,
    ) -> AppSyncResolverResult<Vec<Value>> {
        events
            .into_iter()
            .map(|event| self.handle(&event))
            .collect()
    }

    fn route_for(&self, event: &AppSyncEvent) -> AppSyncResolverResult<&AppSyncRoute> {
        let type_name = event.info.parent_type_name.as_str();
        let field_name = event.info.field_name.as_str();

        self.routes
            .iter()
            .filter_map(|route| {
                route
                    .match_score(type_name, field_name)
                    .map(|score| (score, route))
            })
            .max_by_key(|(score, _)| *score)
            .map(|(_, route)| route)
            .ok_or_else(|| AppSyncResolverError::NoRoute {
                type_name: type_name.to_owned(),
                field_name: field_name.to_owned(),
            })
    }
}

impl AsyncAppSyncResolver {
    /// Creates an empty asynchronous `AppSync` resolver.
    #[must_use]
    pub const fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Registers an asynchronous resolver handler for a GraphQL type and field.
    pub fn resolver(
        &mut self,
        type_name: impl Into<String>,
        field_name: impl Into<String>,
        handler: impl for<'a> Fn(&'a AppSyncEvent) -> AppSyncResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.routes
            .push(AsyncAppSyncRoute::new(type_name, field_name, handler));
        self
    }

    /// Registers an asynchronous resolver handler for a `Query` field.
    pub fn query(
        &mut self,
        field_name: impl Into<String>,
        handler: impl for<'a> Fn(&'a AppSyncEvent) -> AppSyncResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.resolver("Query", field_name, handler)
    }

    /// Registers an asynchronous resolver handler for a `Mutation` field.
    pub fn mutation(
        &mut self,
        field_name: impl Into<String>,
        handler: impl for<'a> Fn(&'a AppSyncEvent) -> AppSyncResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.resolver("Mutation", field_name, handler)
    }

    /// Registers an asynchronous resolver handler for any GraphQL parent type with this field.
    pub fn field(
        &mut self,
        field_name: impl Into<String>,
        handler: impl for<'a> Fn(&'a AppSyncEvent) -> AppSyncResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.resolver("*", field_name, handler)
    }

    /// Returns registered asynchronous resolver routes in insertion order.
    #[must_use]
    pub fn routes(&self) -> &[AsyncAppSyncRoute] {
        &self.routes
    }

    /// Dispatches an `AppSync` direct resolver event to a registered async route.
    ///
    /// Exact `type.field` routes take precedence over wildcard type routes.
    ///
    /// # Errors
    ///
    /// Returns [`AppSyncResolverError`] when no route is registered for the
    /// event's GraphQL parent type and field.
    pub async fn handle(&self, event: &AppSyncEvent) -> AppSyncResolverResult<Value> {
        let route = self.route_for(event)?;
        Ok(route.handle(event).await)
    }

    /// Dispatches batched `AppSync` direct resolver events in input order.
    ///
    /// # Errors
    ///
    /// Returns [`AppSyncResolverError`] when any event cannot be routed.
    pub async fn handle_batch(
        &self,
        events: impl IntoIterator<Item = AppSyncEvent>,
    ) -> AppSyncResolverResult<Vec<Value>> {
        let mut responses = Vec::new();

        for event in events {
            responses.push(self.handle(&event).await?);
        }

        Ok(responses)
    }

    fn route_for(&self, event: &AppSyncEvent) -> AppSyncResolverResult<&AsyncAppSyncRoute> {
        let type_name = event.info.parent_type_name.as_str();
        let field_name = event.info.field_name.as_str();

        self.routes
            .iter()
            .filter_map(|route| {
                route
                    .match_score(type_name, field_name)
                    .map(|score| (score, route))
            })
            .max_by_key(|(score, _)| *score)
            .map(|(_, route)| route)
            .ok_or_else(|| AppSyncResolverError::NoRoute {
                type_name: type_name.to_owned(),
                field_name: field_name.to_owned(),
            })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{AppSyncEvent, AppSyncResolver, AppSyncResolverError, AsyncAppSyncResolver};

    #[test]
    fn routes_exact_appsync_direct_resolver_event() {
        let mut resolver = AppSyncResolver::new();
        resolver.query("getOrder", |event| {
            json!({
                "id": event.arguments.as_ref().and_then(|value| value.get("id")),
                "status": "ok",
            })
        });

        let response = resolver
            .handle(&event("Query", "getOrder", &json!({ "id": "order-1" })))
            .expect("route should match");

        assert_eq!(
            response,
            json!({
                "id": "order-1",
                "status": "ok",
            })
        );
    }

    #[test]
    fn exact_route_precedes_wildcard_type_route() {
        let mut resolver = AppSyncResolver::new();
        resolver.field("name", |_| json!("wildcard"));
        resolver.resolver("Product", "name", |_| json!("product"));

        let response = resolver
            .handle(&event("Product", "name", &Value::Null))
            .expect("route should match");

        assert_eq!(response, json!("product"));
    }

    #[test]
    fn batch_dispatch_preserves_event_order() {
        let mut resolver = AppSyncResolver::new();
        resolver.field("name", |event| {
            event
                .source
                .as_ref()
                .and_then(|value| value.get("name"))
                .cloned()
                .unwrap_or(Value::Null)
        });

        let events = [
            event_with_source("Product", "name", &json!({ "name": "first" })),
            event_with_source("Customer", "name", &json!({ "name": "second" })),
        ];

        let response = resolver
            .handle_batch(events)
            .expect("routes should match in order");

        assert_eq!(response, vec![json!("first"), json!("second")]);
    }

    #[test]
    fn missing_route_returns_type_and_field() {
        let resolver = AppSyncResolver::new();

        let error = resolver
            .handle(&event("Query", "missing", &Value::Null))
            .expect_err("missing route should fail");

        assert_eq!(
            error,
            AppSyncResolverError::NoRoute {
                type_name: "Query".to_owned(),
                field_name: "missing".to_owned(),
            }
        );
    }

    #[test]
    fn async_resolver_routes_exact_appsync_direct_resolver_event() {
        let mut resolver = AsyncAppSyncResolver::new();
        resolver.query("getOrder", |event| {
            Box::pin(async move {
                json!({
                    "id": event.arguments.as_ref().and_then(|value| value.get("id")),
                    "status": "ok",
                })
            })
        });
        let event = event("Query", "getOrder", &json!({ "id": "order-1" }));

        let response =
            futures_executor::block_on(resolver.handle(&event)).expect("route should match");

        assert_eq!(
            response,
            json!({
                "id": "order-1",
                "status": "ok",
            })
        );
    }

    #[test]
    fn async_exact_route_precedes_wildcard_type_route() {
        let mut resolver = AsyncAppSyncResolver::new();
        resolver.field("name", |_| Box::pin(async { json!("wildcard") }));
        resolver.resolver("Product", "name", |_| Box::pin(async { json!("product") }));
        let event = event("Product", "name", &Value::Null);

        let response =
            futures_executor::block_on(resolver.handle(&event)).expect("route should match");

        assert_eq!(response, json!("product"));
    }

    #[test]
    fn async_batch_dispatch_preserves_event_order() {
        let mut resolver = AsyncAppSyncResolver::new();
        resolver.field("name", |event| {
            Box::pin(async move {
                event
                    .source
                    .as_ref()
                    .and_then(|value| value.get("name"))
                    .cloned()
                    .unwrap_or(Value::Null)
            })
        });

        let events = [
            event_with_source("Product", "name", &json!({ "name": "first" })),
            event_with_source("Customer", "name", &json!({ "name": "second" })),
        ];

        let response = futures_executor::block_on(resolver.handle_batch(events))
            .expect("routes should match in order");

        assert_eq!(response, vec![json!("first"), json!("second")]);
    }

    #[test]
    fn async_missing_route_returns_type_and_field() {
        let resolver = AsyncAppSyncResolver::new();
        let event = event("Query", "missing", &Value::Null);

        let error = futures_executor::block_on(resolver.handle(&event))
            .expect_err("missing route should fail");

        assert_eq!(
            error,
            AppSyncResolverError::NoRoute {
                type_name: "Query".to_owned(),
                field_name: "missing".to_owned(),
            }
        );
    }

    fn event(type_name: &str, field_name: &str, arguments: &Value) -> AppSyncEvent {
        event_value(type_name, field_name, arguments, &Value::Null)
    }

    fn event_with_source(type_name: &str, field_name: &str, source: &Value) -> AppSyncEvent {
        event_value(type_name, field_name, &Value::Null, source)
    }

    fn event_value(
        type_name: &str,
        field_name: &str,
        arguments: &Value,
        source: &Value,
    ) -> AppSyncEvent {
        serde_json::from_value(json!({
            "arguments": arguments,
            "identity": null,
            "info": {
                "fieldName": field_name,
                "parentTypeName": type_name,
                "selectionSetGraphQL": "",
                "selectionSetList": [],
                "variables": {},
            },
            "prev": null,
            "request": {
                "domainName": null,
                "headers": {},
            },
            "source": source,
            "stash": {},
        }))
        .expect("AppSync direct resolver event should deserialize")
    }
}
