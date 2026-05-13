//! AWS `AppSync` Events routing.

use std::fmt;

use aws_lambda_powertools_parser::{
    AppSyncEventsEvent, AppSyncEventsIncomingEvent, AppSyncEventsOperation,
};
use serde_json::{Value, json};

/// Handler result for AWS `AppSync` Events route handlers.
pub type AppSyncEventsHandlerResult<T> = Result<T, AppSyncEventsHandlerError>;

/// Handler function for one AWS `AppSync` Events published payload.
pub type AppSyncEventsPublishHandler = dyn Fn(&Value, &AppSyncEventsEvent) -> AppSyncEventsHandlerResult<Value>
    + Send
    + Sync
    + 'static;

/// Handler function for all AWS `AppSync` Events published messages in one call.
pub type AppSyncEventsAggregatePublishHandler = dyn Fn(
        &[AppSyncEventsIncomingEvent],
        &AppSyncEventsEvent,
    ) -> AppSyncEventsHandlerResult<Vec<AppSyncEventsIncomingEvent>>
    + Send
    + Sync
    + 'static;

/// Handler function for AWS `AppSync` Events subscribe operations.
pub type AppSyncEventsSubscribeHandler =
    dyn Fn(&AppSyncEventsEvent) -> AppSyncEventsHandlerResult<()> + Send + Sync + 'static;

/// Error returned by AWS `AppSync` Events route handlers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSyncEventsHandlerError {
    message: String,
}

impl AppSyncEventsHandlerError {
    /// Creates a route handler error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AppSyncEventsHandlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppSyncEventsHandlerError {}

impl From<&str> for AppSyncEventsHandlerError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

impl From<String> for AppSyncEventsHandlerError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

/// Error returned when an AWS `AppSync` Events invocation cannot be resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppSyncEventsResolverError {
    /// A publish invocation did not include published events.
    MissingPublishEvents,
}

impl fmt::Display for AppSyncEventsResolverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPublishEvents => {
                formatter.write_str("AppSync Events publish event is missing events")
            }
        }
    }
}

impl std::error::Error for AppSyncEventsResolverError {}

/// Result returned by AWS `AppSync` Events resolver dispatch.
pub type AppSyncEventsResolverResult<T> = Result<T, AppSyncEventsResolverError>;

/// Registered AWS `AppSync` Events publish route.
pub struct AppSyncEventsPublishRoute {
    path: String,
    pattern: AppSyncEventsPathPattern,
    handler: AppSyncEventsPublishRouteHandler,
}

enum AppSyncEventsPublishRouteHandler {
    Each(Box<AppSyncEventsPublishHandler>),
    Aggregate(Box<AppSyncEventsAggregatePublishHandler>),
}

impl AppSyncEventsPublishRoute {
    /// Creates a publish route that handles each event payload separately.
    #[must_use]
    pub fn new(
        path: impl Into<String>,
        handler: impl Fn(&Value, &AppSyncEventsEvent) -> AppSyncEventsHandlerResult<Value>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        let path = path.into();

        Self {
            pattern: AppSyncEventsPathPattern::parse(&path),
            path,
            handler: AppSyncEventsPublishRouteHandler::Each(Box::new(handler)),
        }
    }

    /// Creates a publish route that handles all published events at once.
    #[must_use]
    pub fn aggregate(
        path: impl Into<String>,
        handler: impl Fn(
            &[AppSyncEventsIncomingEvent],
            &AppSyncEventsEvent,
        ) -> AppSyncEventsHandlerResult<Vec<AppSyncEventsIncomingEvent>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        let path = path.into();

        Self {
            pattern: AppSyncEventsPathPattern::parse(&path),
            path,
            handler: AppSyncEventsPublishRouteHandler::Aggregate(Box::new(handler)),
        }
    }

    /// Returns the registered channel path pattern.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns true when this route handles all events in one call.
    #[must_use]
    pub const fn is_aggregate(&self) -> bool {
        matches!(self.handler, AppSyncEventsPublishRouteHandler::Aggregate(_))
    }

    fn match_score(&self, path: &str) -> Option<usize> {
        self.pattern.match_score(path)
    }

    fn handle(&self, event: &AppSyncEventsEvent) -> AppSyncEventsResolverResult<Value> {
        let events = event
            .events
            .as_ref()
            .ok_or(AppSyncEventsResolverError::MissingPublishEvents)?;

        match &self.handler {
            AppSyncEventsPublishRouteHandler::Each(handler) => {
                let events = events
                    .iter()
                    .map(|message| match handler(&message.payload, event) {
                        Ok(payload) => json!({
                            "id": &message.id,
                            "payload": payload,
                        }),
                        Err(error) => json!({
                            "id": &message.id,
                            "error": error.to_string(),
                        }),
                    })
                    .collect::<Vec<_>>();

                Ok(json!({ "events": events }))
            }
            AppSyncEventsPublishRouteHandler::Aggregate(handler) => match handler(events, event) {
                Ok(events) => Ok(json!({ "events": events })),
                Err(error) => Ok(json!({ "error": error.to_string() })),
            },
        }
    }
}

impl fmt::Debug for AppSyncEventsPublishRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppSyncEventsPublishRoute")
            .field("path", &self.path)
            .field("aggregate", &self.is_aggregate())
            .finish_non_exhaustive()
    }
}

/// Registered AWS `AppSync` Events subscribe route.
pub struct AppSyncEventsSubscribeRoute {
    path: String,
    pattern: AppSyncEventsPathPattern,
    handler: Box<AppSyncEventsSubscribeHandler>,
}

impl AppSyncEventsSubscribeRoute {
    /// Creates a subscribe route.
    #[must_use]
    pub fn new(
        path: impl Into<String>,
        handler: impl Fn(&AppSyncEventsEvent) -> AppSyncEventsHandlerResult<()> + Send + Sync + 'static,
    ) -> Self {
        let path = path.into();

        Self {
            pattern: AppSyncEventsPathPattern::parse(&path),
            path,
            handler: Box::new(handler),
        }
    }

    /// Returns the registered channel path pattern.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    fn match_score(&self, path: &str) -> Option<usize> {
        self.pattern.match_score(path)
    }

    fn handle(&self, event: &AppSyncEventsEvent) -> Value {
        match (self.handler)(event) {
            Ok(()) => Value::Null,
            Err(error) => json!({ "error": error.to_string() }),
        }
    }
}

impl fmt::Debug for AppSyncEventsSubscribeRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppSyncEventsSubscribeRoute")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Routes AWS `AppSync` Events publish and subscribe invocations by channel path.
#[derive(Default, Debug)]
pub struct AppSyncEventsResolver {
    publish_routes: Vec<AppSyncEventsPublishRoute>,
    subscribe_routes: Vec<AppSyncEventsSubscribeRoute>,
}

impl AppSyncEventsResolver {
    /// Creates an empty AWS `AppSync` Events resolver.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            publish_routes: Vec::new(),
            subscribe_routes: Vec::new(),
        }
    }

    /// Registers a publish route that handles each event payload separately.
    pub fn on_publish(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Value, &AppSyncEventsEvent) -> AppSyncEventsHandlerResult<Value>
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.publish_routes
            .push(AppSyncEventsPublishRoute::new(path, handler));
        self
    }

    /// Registers a publish route that handles all published events at once.
    pub fn on_publish_aggregate(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(
            &[AppSyncEventsIncomingEvent],
            &AppSyncEventsEvent,
        ) -> AppSyncEventsHandlerResult<Vec<AppSyncEventsIncomingEvent>>
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.publish_routes
            .push(AppSyncEventsPublishRoute::aggregate(path, handler));
        self
    }

    /// Registers a subscribe route.
    pub fn on_subscribe(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&AppSyncEventsEvent) -> AppSyncEventsHandlerResult<()> + Send + Sync + 'static,
    ) -> &mut Self {
        self.subscribe_routes
            .push(AppSyncEventsSubscribeRoute::new(path, handler));
        self
    }

    /// Returns registered publish routes in insertion order.
    #[must_use]
    pub fn publish_routes(&self) -> &[AppSyncEventsPublishRoute] {
        &self.publish_routes
    }

    /// Returns registered subscribe routes in insertion order.
    #[must_use]
    pub fn subscribe_routes(&self) -> &[AppSyncEventsSubscribeRoute] {
        &self.subscribe_routes
    }

    /// Dispatches an AWS `AppSync` Events invocation.
    ///
    /// Publish handlers return an object with an `events` array, or an `error`
    /// field for aggregate handler failures. Subscribe handlers return `null`
    /// on success and an object with an `error` field on failure.
    ///
    /// # Errors
    ///
    /// Returns [`AppSyncEventsResolverError`] when a publish invocation is
    /// missing its `events` collection.
    pub fn handle(&self, event: &AppSyncEventsEvent) -> AppSyncEventsResolverResult<Value> {
        match event.info.operation {
            AppSyncEventsOperation::Publish => self.handle_publish(event),
            AppSyncEventsOperation::Subscribe => Ok(self.handle_subscribe(event)),
        }
    }

    /// Dispatches an AWS `AppSync` Events publish invocation.
    ///
    /// When no route matches, the original published events are returned
    /// unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`AppSyncEventsResolverError::MissingPublishEvents`] when the
    /// invocation does not include published events.
    pub fn handle_publish(&self, event: &AppSyncEventsEvent) -> AppSyncEventsResolverResult<Value> {
        let route = self.route_for_publish(event.info.channel.path.as_str());

        if let Some(route) = route {
            return route.handle(event);
        }

        let events = event
            .events
            .as_ref()
            .ok_or(AppSyncEventsResolverError::MissingPublishEvents)?;
        Ok(json!({ "events": events }))
    }

    /// Dispatches an AWS `AppSync` Events subscribe invocation.
    ///
    /// When no route matches, the response is `null`, which accepts the
    /// subscription.
    #[must_use]
    pub fn handle_subscribe(&self, event: &AppSyncEventsEvent) -> Value {
        self.route_for_subscribe(event.info.channel.path.as_str())
            .map_or(Value::Null, |route| route.handle(event))
    }

    fn route_for_publish(&self, path: &str) -> Option<&AppSyncEventsPublishRoute> {
        best_route(self.publish_routes.iter(), path, |route, path| {
            route.match_score(path)
        })
    }

    fn route_for_subscribe(&self, path: &str) -> Option<&AppSyncEventsSubscribeRoute> {
        best_route(self.subscribe_routes.iter(), path, |route, path| {
            route.match_score(path)
        })
    }
}

fn best_route<'a, T>(
    routes: impl Iterator<Item = &'a T>,
    path: &str,
    mut score: impl FnMut(&T, &str) -> Option<usize>,
) -> Option<&'a T> {
    let mut best = None;
    let mut best_score = 0;

    for route in routes {
        let Some(route_score) = score(route, path) else {
            continue;
        };

        if best.is_none() || route_score > best_score {
            best = Some(route);
            best_score = route_score;
        }
    }

    best
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AppSyncEventsPathPattern {
    path: String,
    valid: bool,
    wildcard: bool,
}

impl AppSyncEventsPathPattern {
    fn parse(path: &str) -> Self {
        let valid = is_valid_appsync_events_path(path);
        let wildcard = valid && path.ends_with('*');

        Self {
            path: path.to_owned(),
            valid,
            wildcard,
        }
    }

    fn match_score(&self, path: &str) -> Option<usize> {
        if !self.valid {
            return None;
        }

        if self.path == path {
            return Some(self.path.len());
        }

        if !self.wildcard {
            return None;
        }

        let prefix = self.path.trim_end_matches('*');

        path.starts_with(prefix).then_some(prefix.len())
    }
}

fn is_valid_appsync_events_path(path: &str) -> bool {
    if path == "/*" {
        return true;
    }

    if !path.starts_with('/') || path.ends_with('/') {
        return false;
    }

    let mut seen_wildcard = false;
    let mut segment_count = 0;

    for segment in path.trim_start_matches('/').split('/') {
        segment_count += 1;
        if segment.is_empty() {
            return false;
        }

        if seen_wildcard {
            return false;
        }

        if segment == "*" {
            seen_wildcard = true;
            continue;
        }

        if segment.contains('*') {
            return false;
        }
    }

    segment_count > 0
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use aws_lambda_powertools_parser::{
        AppSyncEventsChannel, AppSyncEventsChannelNamespace, AppSyncEventsEvent,
        AppSyncEventsIncomingEvent, AppSyncEventsInfo, AppSyncEventsOperation,
        AppSyncEventsRequest,
    };
    use serde_json::{Value, json};

    use super::{
        AppSyncEventsHandlerError, AppSyncEventsResolver, AppSyncEventsResolverError,
        is_valid_appsync_events_path,
    };

    #[test]
    fn publish_without_route_returns_events_unchanged() {
        let event = publish_event(
            "/default/orders",
            vec![message("event-1", json!({"id": 1}))],
        );

        let response = AppSyncEventsResolver::new()
            .handle(&event)
            .expect("publish response should build");

        assert_eq!(
            response,
            json!({
                "events": [
                    {
                        "id": "event-1",
                        "payload": {
                            "id": 1
                        }
                    }
                ]
            })
        );
    }

    #[test]
    fn publish_route_processes_each_payload() {
        let event = publish_event(
            "/default/orders",
            vec![
                message("event-1", json!({"id": 1})),
                message("event-2", json!({"id": 2})),
            ],
        );
        let mut resolver = AppSyncEventsResolver::new();
        resolver.on_publish("/default/orders", |payload, _| {
            Ok(json!({
                "processed": true,
                "id": payload["id"],
            }))
        });

        let response = resolver
            .handle_publish(&event)
            .expect("publish response should build");

        assert_eq!(
            response,
            json!({
                "events": [
                    {
                        "id": "event-1",
                        "payload": {
                            "processed": true,
                            "id": 1
                        }
                    },
                    {
                        "id": "event-2",
                        "payload": {
                            "processed": true,
                            "id": 2
                        }
                    }
                ]
            })
        );
    }

    #[test]
    fn publish_route_formats_per_event_errors() {
        let event = publish_event(
            "/default/orders",
            vec![
                message("event-1", json!({"id": 1})),
                message("event-2", json!({"fail": true})),
            ],
        );
        let mut resolver = AppSyncEventsResolver::new();
        resolver.on_publish("/default/orders", |payload, _| {
            if payload
                .get("fail")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return Err(AppSyncEventsHandlerError::new("invalid event"));
            }

            Ok(payload.clone())
        });

        let response = resolver
            .handle_publish(&event)
            .expect("publish response should build");

        assert_eq!(
            response,
            json!({
                "events": [
                    {
                        "id": "event-1",
                        "payload": {
                            "id": 1
                        }
                    },
                    {
                        "id": "event-2",
                        "error": "invalid event"
                    }
                ]
            })
        );
    }

    #[test]
    fn aggregate_publish_route_processes_all_events() {
        let event = publish_event(
            "/default/orders",
            vec![
                message("event-1", json!({"id": 1})),
                message("event-2", json!({"id": 2})),
            ],
        );
        let mut resolver = AppSyncEventsResolver::new();
        resolver.on_publish_aggregate("/default/orders", |events, _| {
            Ok(events
                .iter()
                .map(|event| {
                    message(
                        &event.id,
                        json!({
                            "batch": true,
                            "payload": &event.payload,
                        }),
                    )
                })
                .collect())
        });

        let response = resolver
            .handle_publish(&event)
            .expect("publish response should build");

        assert_eq!(
            response,
            json!({
                "events": [
                    {
                        "id": "event-1",
                        "payload": {
                            "batch": true,
                            "payload": {
                                "id": 1
                            }
                        }
                    },
                    {
                        "id": "event-2",
                        "payload": {
                            "batch": true,
                            "payload": {
                                "id": 2
                            }
                        }
                    }
                ]
            })
        );
    }

    #[test]
    fn aggregate_publish_route_formats_handler_error() {
        let event = publish_event(
            "/default/orders",
            vec![message("event-1", json!({"id": 1}))],
        );
        let mut resolver = AppSyncEventsResolver::new();
        resolver.on_publish_aggregate("/default/orders", |_, _| {
            Err(AppSyncEventsHandlerError::new("batch failed"))
        });

        let response = resolver
            .handle_publish(&event)
            .expect("publish response should build");

        assert_eq!(response, json!({ "error": "batch failed" }));
    }

    #[test]
    fn publish_route_uses_most_specific_path() {
        let event = publish_event(
            "/default/orders/priority",
            vec![message("event-1", json!({"id": 1}))],
        );
        let mut resolver = AppSyncEventsResolver::new();
        resolver.on_publish("/default/*", |_, _| Ok(json!({"route": "wildcard"})));
        resolver.on_publish("/default/orders/*", |_, _| Ok(json!({"route": "specific"})));

        let response = resolver
            .handle_publish(&event)
            .expect("publish response should build");

        assert_eq!(response["events"][0]["payload"]["route"], "specific");
    }

    #[test]
    fn subscribe_without_route_returns_null() {
        let event = subscribe_event("/default/orders");

        let response = AppSyncEventsResolver::new().handle_subscribe(&event);

        assert_eq!(response, Value::Null);
    }

    #[test]
    fn subscribe_route_accepts_subscription() {
        let event = subscribe_event("/default/orders");
        let mut resolver = AppSyncEventsResolver::new();
        resolver.on_subscribe("/default/orders", |_| Ok(()));

        let response = resolver.handle(&event).expect("subscribe should resolve");

        assert_eq!(response, Value::Null);
    }

    #[test]
    fn subscribe_route_formats_handler_error() {
        let event = subscribe_event("/default/orders");
        let mut resolver = AppSyncEventsResolver::new();
        resolver.on_subscribe("/default/orders", |_| {
            Err(AppSyncEventsHandlerError::new("not authorized"))
        });

        let response = resolver.handle_subscribe(&event);

        assert_eq!(response, json!({ "error": "not authorized" }));
    }

    #[test]
    fn invalid_routes_do_not_match() {
        let event = subscribe_event("/default/orders");
        let mut resolver = AppSyncEventsResolver::new();
        resolver.on_subscribe("default/*", |_| {
            Err(AppSyncEventsHandlerError::new("should not run"))
        });

        let response = resolver.handle_subscribe(&event);

        assert_eq!(response, Value::Null);
    }

    #[test]
    fn missing_publish_events_return_error() {
        let mut event = publish_event("/default/orders", Vec::new());
        event.events = None;

        let error = AppSyncEventsResolver::new()
            .handle_publish(&event)
            .expect_err("missing publish events should fail");

        assert_eq!(error, AppSyncEventsResolverError::MissingPublishEvents);
    }

    #[test]
    fn validates_appsync_events_paths() {
        assert!(is_valid_appsync_events_path("/*"));
        assert!(is_valid_appsync_events_path("/default"));
        assert!(is_valid_appsync_events_path("/default/orders/*"));
        assert!(!is_valid_appsync_events_path("default/orders"));
        assert!(!is_valid_appsync_events_path("/default/*/orders"));
        assert!(!is_valid_appsync_events_path("/default/orders*"));
        assert!(!is_valid_appsync_events_path("/default/"));
    }

    fn publish_event(path: &str, events: Vec<AppSyncEventsIncomingEvent>) -> AppSyncEventsEvent {
        appsync_events_event(path, AppSyncEventsOperation::Publish, Some(events))
    }

    fn subscribe_event(path: &str) -> AppSyncEventsEvent {
        appsync_events_event(path, AppSyncEventsOperation::Subscribe, None)
    }

    fn appsync_events_event(
        path: &str,
        operation: AppSyncEventsOperation,
        events: Option<Vec<AppSyncEventsIncomingEvent>>,
    ) -> AppSyncEventsEvent {
        AppSyncEventsEvent {
            identity: None,
            result: None,
            request: AppSyncEventsRequest {
                headers: BTreeMap::default(),
                domain_name: None,
            },
            info: AppSyncEventsInfo {
                channel: AppSyncEventsChannel {
                    path: path.to_owned(),
                    segments: path
                        .trim_start_matches('/')
                        .split('/')
                        .map(str::to_owned)
                        .collect(),
                },
                channel_namespace: AppSyncEventsChannelNamespace {
                    name: "default".to_owned(),
                },
                operation,
            },
            error: None,
            prev: None,
            stash: BTreeMap::default(),
            out_errors: Some(Vec::new()),
            events,
        }
    }

    fn message(id: &str, payload: Value) -> AppSyncEventsIncomingEvent {
        AppSyncEventsIncomingEvent {
            id: id.to_owned(),
            payload,
        }
    }
}
