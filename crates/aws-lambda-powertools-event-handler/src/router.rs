//! Router facade.

use std::{any::TypeId, cmp::Ordering, fmt, future::Future, pin::Pin, sync::Arc};

use aws_lambda_powertools_core::env;

#[cfg(feature = "validation")]
use crate::validation::{
    ValidationConfig, request_validation_response, response_validation_response,
};
use crate::{
    AsyncFallibleHandler, AsyncHandler, AsyncRoute, CorsConfig, Extensions, FallibleHandler,
    FallibleResponseFuture, Handler, HttpError, Method, Request, Response, ResponseFuture, Route,
    RouteError,
};

/// Function signature used by request middleware.
pub type RequestMiddleware = dyn Fn(Request) -> Request + Send + Sync + 'static;

/// Function signature used by response middleware.
pub type ResponseMiddleware = dyn Fn(&Request, Response) -> Response + Send + Sync + 'static;

/// Function signature used by fallible route error handlers.
pub type ErrorHandler = dyn Fn(&Request, &RouteError) -> Response + Send + Sync + 'static;

/// Function signature used by asynchronous fallible route error handlers.
pub type AsyncErrorHandler =
    dyn for<'a> Fn(&'a Request, &'a RouteError) -> ResponseFuture<'a> + Send + Sync + 'static;

type OptionalResponseFuture<'a> = Pin<Box<dyn Future<Output = Option<Response>> + Send + 'a>>;
type TypedErrorHandler = dyn Fn(&Request, &RouteError) -> Option<Response> + Send + Sync + 'static;
type AsyncTypedErrorHandler = dyn for<'a> Fn(&'a Request, &'a RouteError) -> OptionalResponseFuture<'a>
    + Send
    + Sync
    + 'static;

/// Stores HTTP route handlers and selects the most specific matching route.
///
/// Route precedence is path-first: static path segments take precedence over
/// dynamic path parameters, then exact method routes take precedence over
/// `Method::Any`. Ties preserve registration order.
#[derive(Default)]
pub struct Router {
    routes: Vec<Route>,
    cors: Option<CorsConfig>,
    request_middleware: Vec<Box<RequestMiddleware>>,
    response_middleware: Vec<Box<ResponseMiddleware>>,
    shared_extensions: Extensions,
    not_found_handler: Option<Box<Handler>>,
    typed_error_handlers: Vec<TypedErrorHandlerEntry>,
    error_handler: Option<Box<ErrorHandler>>,
    debug_errors: Option<bool>,
    #[cfg(feature = "validation")]
    validation: Option<ValidationConfig>,
}

/// Stores asynchronous HTTP route handlers and selects the most specific matching route.
///
/// Route precedence is path-first: static path segments take precedence over
/// dynamic path parameters, then exact method routes take precedence over
/// `Method::Any`. Ties preserve registration order.
#[derive(Default)]
pub struct AsyncRouter {
    routes: Vec<AsyncRoute>,
    cors: Option<CorsConfig>,
    request_middleware: Vec<Box<RequestMiddleware>>,
    response_middleware: Vec<Box<ResponseMiddleware>>,
    shared_extensions: Extensions,
    not_found_handler: Option<Box<AsyncHandler>>,
    typed_error_handlers: Vec<AsyncTypedErrorHandlerEntry>,
    error_handler: Option<Box<AsyncErrorHandler>>,
    debug_errors: Option<bool>,
    #[cfg(feature = "validation")]
    validation: Option<ValidationConfig>,
}

impl Router {
    /// Creates an empty router.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            routes: Vec::new(),
            cors: None,
            request_middleware: Vec::new(),
            response_middleware: Vec::new(),
            shared_extensions: Extensions::new(),
            not_found_handler: None,
            typed_error_handlers: Vec::new(),
            error_handler: None,
            debug_errors: None,
            #[cfg(feature = "validation")]
            validation: None,
        }
    }

    /// Enables CORS handling with the provided configuration.
    pub fn enable_cors(&mut self, cors: CorsConfig) -> &mut Self {
        self.cors = Some(cors);
        self
    }

    /// Returns a copy of this router with CORS enabled.
    #[must_use]
    pub fn with_cors(mut self, cors: CorsConfig) -> Self {
        self.cors = Some(cors);
        self
    }

    /// Returns the CORS configuration when enabled.
    #[must_use]
    pub const fn cors(&self) -> Option<&CorsConfig> {
        self.cors.as_ref()
    }

    /// Returns whether default internal-error responses expose error details.
    ///
    /// When not explicitly configured, this follows `POWERTOOLS_DEV`.
    #[must_use]
    pub fn debug_errors(&self) -> bool {
        self.debug_errors.unwrap_or_else(powertools_dev_enabled)
    }

    /// Sets whether default internal-error responses expose error details.
    ///
    /// This only affects unhandled fallible route errors. Built-in
    /// [`HttpError`] responses and custom error handlers keep their existing
    /// response bodies.
    pub fn set_debug_errors(&mut self, enabled: bool) -> &mut Self {
        self.debug_errors = Some(enabled);
        self
    }

    /// Returns a copy of this router with default error detail rendering configured.
    #[must_use]
    pub fn with_debug_errors(mut self, enabled: bool) -> Self {
        self.set_debug_errors(enabled);
        self
    }

    /// Returns router shared extension values.
    #[must_use]
    pub const fn shared_extensions(&self) -> &Extensions {
        &self.shared_extensions
    }

    /// Returns a router shared extension value by type.
    #[must_use]
    pub fn shared_extension<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        self.shared_extensions.get::<T>()
    }

    /// Adds or replaces a router shared extension value.
    pub fn insert_shared_extension<T>(&mut self, value: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.shared_extensions.insert(value);
        self
    }

    /// Returns a copy of this router with a router shared extension value.
    #[must_use]
    pub fn with_shared_extension<T>(mut self, value: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        self.insert_shared_extension(value);
        self
    }

    /// Removes all router shared extension values.
    pub fn clear_shared_extensions(&mut self) -> &mut Self {
        self.shared_extensions.clear();
        self
    }

    /// Adds request middleware that runs before CORS preflight handling and route matching.
    pub fn add_request_middleware(
        &mut self,
        middleware: impl Fn(Request) -> Request + Send + Sync + 'static,
    ) -> &mut Self {
        self.request_middleware.push(Box::new(middleware));
        self
    }

    /// Adds response middleware that runs after route handling and before CORS headers are applied.
    pub fn add_response_middleware(
        &mut self,
        middleware: impl Fn(&Request, Response) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.response_middleware.push(Box::new(middleware));
        self
    }

    /// Adds request and response middleware that records per-request HTTP metrics.
    #[cfg(feature = "metrics")]
    pub fn add_metrics_middleware(
        &mut self,
        metrics: std::sync::Arc<std::sync::Mutex<aws_lambda_powertools_metrics::Metrics>>,
    ) -> &mut Self {
        self.add_request_middleware(crate::http_metrics_start_middleware());
        self.add_response_middleware(crate::http_metrics_response_middleware(metrics));
        self
    }

    /// Adds response middleware that records per-request HTTP trace segments.
    #[cfg(feature = "tracer")]
    pub fn add_trace_middleware(
        &mut self,
        tracer: aws_lambda_powertools_tracer::Tracer,
        sink: std::sync::Arc<crate::HttpTraceSink>,
    ) -> &mut Self {
        self.add_response_middleware(crate::http_trace_response_middleware(tracer, sink));
        self
    }

    /// Returns the number of registered request middleware functions.
    #[must_use]
    pub fn request_middleware_len(&self) -> usize {
        self.request_middleware.len()
    }

    /// Returns the number of registered response middleware functions.
    #[must_use]
    pub fn response_middleware_len(&self) -> usize {
        self.response_middleware.len()
    }

    /// Sets the handler used when no route matches a request.
    ///
    /// Response middleware and CORS handling still run for custom not-found
    /// responses.
    pub fn set_not_found_handler(
        &mut self,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.not_found_handler = Some(Box::new(handler));
        self
    }

    /// Returns a copy of this router with a custom not-found handler.
    #[must_use]
    pub fn with_not_found_handler(
        mut self,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> Self {
        self.set_not_found_handler(handler);
        self
    }

    /// Sets the handler used when a fallible route returns an error.
    ///
    /// This handler is only used for routes registered with
    /// [`Router::add_fallible_route`] or the `*_fallible` route helpers. It
    /// does not catch panics. Response middleware and CORS handling still run
    /// for custom error responses.
    pub fn set_error_handler(
        &mut self,
        handler: impl Fn(&Request, &RouteError) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.error_handler = Some(Box::new(handler));
        self
    }

    /// Sets the handler used when a fallible route returns a concrete error type.
    ///
    /// Typed error handlers run before the catch-all handler configured with
    /// [`Router::set_error_handler`]. Registering another handler for the same
    /// error type replaces the previous handler.
    pub fn set_error_handler_for<E>(
        &mut self,
        handler: impl Fn(&Request, &E) -> Response + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.upsert_typed_error_handler(TypedErrorHandlerEntry::new(handler));
        self
    }

    /// Returns a copy of this router with a fallible route error handler.
    #[must_use]
    pub fn with_error_handler(
        mut self,
        handler: impl Fn(&Request, &RouteError) -> Response + Send + Sync + 'static,
    ) -> Self {
        self.set_error_handler(handler);
        self
    }

    /// Returns a copy of this router with a concrete fallible route error handler.
    #[must_use]
    pub fn with_error_handler_for<E>(
        mut self,
        handler: impl Fn(&Request, &E) -> Response + Send + Sync + 'static,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.set_error_handler_for(handler);
        self
    }

    /// Enables validation with the provided configuration.
    #[cfg(feature = "validation")]
    pub fn enable_validation(&mut self, validation: ValidationConfig) -> &mut Self {
        self.validation = Some(validation);
        self
    }

    /// Returns a copy of this router with validation enabled.
    #[cfg(feature = "validation")]
    #[must_use]
    pub fn with_validation(mut self, validation: ValidationConfig) -> Self {
        self.validation = Some(validation);
        self
    }

    /// Returns the validation configuration when enabled.
    #[cfg(feature = "validation")]
    #[must_use]
    pub const fn validation(&self) -> Option<&ValidationConfig> {
        self.validation.as_ref()
    }

    /// Adds a request validator, enabling validation when needed.
    #[cfg(feature = "validation")]
    pub fn add_request_validator(
        &mut self,
        validator: impl Fn(&Request) -> aws_lambda_powertools_validation::ValidationResult
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.validation
            .get_or_insert_with(ValidationConfig::new)
            .add_request_validator(validator);
        self
    }

    /// Adds a response validator, enabling validation when needed.
    #[cfg(feature = "validation")]
    pub fn add_response_validator(
        &mut self,
        validator: impl Fn(&Request, &Response) -> aws_lambda_powertools_validation::ValidationResult
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.validation
            .get_or_insert_with(ValidationConfig::new)
            .add_response_validator(validator);
        self
    }

    /// Adds a route handler.
    pub fn add_route(
        &mut self,
        method: Method,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.routes.push(Route::new(method, path, handler));
        self
    }

    /// Adds the same route handler for multiple request methods.
    ///
    /// This is a convenience for registering methods such as `GET` and `POST`
    /// on the same path while preserving normal route precedence.
    pub fn add_routes(
        &mut self,
        methods: impl IntoIterator<Item = Method>,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        let path = path.into();
        let handler: Arc<Handler> = Arc::new(handler);

        for method in methods {
            self.routes.push(Route::new_with_handler(
                method,
                path.clone(),
                Arc::clone(&handler),
            ));
        }

        self
    }

    /// Adds a fallible route handler.
    ///
    /// Returned errors are mapped through this router's error handler when one
    /// is configured, otherwise they produce `500 Internal Server Error`.
    pub fn add_fallible_route<E>(
        &mut self,
        method: Method,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.routes.push(Route::new_fallible(method, path, handler));
        self
    }

    /// Adds the same fallible route handler for multiple request methods.
    ///
    /// Returned errors are mapped through this router's error handler when one
    /// is configured, otherwise built-in [`HttpError`] values produce their
    /// status code and other errors produce `500 Internal Server Error`.
    pub fn add_fallible_routes<E>(
        &mut self,
        methods: impl IntoIterator<Item = Method>,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let path = path.into();
        let handler: Arc<FallibleHandler> = Arc::new(move |request| {
            handler(request).map_err(|error| Box::new(error) as Box<RouteError>)
        });

        for method in methods {
            self.routes.push(Route::new_with_fallible_handler(
                method,
                path.clone(),
                Arc::clone(&handler),
            ));
        }

        self
    }

    /// Registers a prebuilt route.
    ///
    /// Use this when the route owns route-specific middleware.
    pub fn register_route(&mut self, route: Route) -> &mut Self {
        self.routes.push(route);
        self
    }

    /// Adds a `GET` route handler.
    pub fn get(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Get, path, handler)
    }

    /// Adds a fallible `GET` route handler.
    pub fn get_fallible<E>(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.add_fallible_route(Method::Get, path, handler)
    }

    /// Adds a `HEAD` route handler.
    pub fn head(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Head, path, handler)
    }

    /// Adds a fallible `HEAD` route handler.
    pub fn head_fallible<E>(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.add_fallible_route(Method::Head, path, handler)
    }

    /// Adds a `POST` route handler.
    pub fn post(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Post, path, handler)
    }

    /// Adds a fallible `POST` route handler.
    pub fn post_fallible<E>(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.add_fallible_route(Method::Post, path, handler)
    }

    /// Adds a `PUT` route handler.
    pub fn put(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Put, path, handler)
    }

    /// Adds a fallible `PUT` route handler.
    pub fn put_fallible<E>(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.add_fallible_route(Method::Put, path, handler)
    }

    /// Adds a `PATCH` route handler.
    pub fn patch(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Patch, path, handler)
    }

    /// Adds a fallible `PATCH` route handler.
    pub fn patch_fallible<E>(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.add_fallible_route(Method::Patch, path, handler)
    }

    /// Adds a `DELETE` route handler.
    pub fn delete(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Delete, path, handler)
    }

    /// Adds a fallible `DELETE` route handler.
    pub fn delete_fallible<E>(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.add_fallible_route(Method::Delete, path, handler)
    }

    /// Adds an `OPTIONS` route handler.
    pub fn options(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Options, path, handler)
    }

    /// Adds a fallible `OPTIONS` route handler.
    pub fn options_fallible<E>(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.add_fallible_route(Method::Options, path, handler)
    }

    /// Adds a route handler that accepts any request method.
    pub fn any(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Any, path, handler)
    }

    /// Adds a fallible route handler that accepts any request method.
    pub fn any_fallible<E>(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.add_fallible_route(Method::Any, path, handler)
    }

    /// Includes routes, middleware, validation hooks, and error handling from another router.
    ///
    /// Included routes behave as if they were registered after this router's
    /// existing routes. The included router's CORS configuration is not merged.
    /// If the included router has a not-found or fallible route error handler,
    /// it replaces this router's matching handler.
    pub fn include_router(&mut self, router: Router) -> &mut Self {
        self.include_router_with_prefix("", router)
    }

    /// Includes another router, prefixing each included route path.
    ///
    /// A prefix of `/api` turns an included route `/orders/{id}` into
    /// `/api/orders/{id}`. Included middleware runs after this router's
    /// existing middleware. The included router's CORS configuration is not
    /// merged.
    pub fn include_router_with_prefix(
        &mut self,
        prefix: impl AsRef<str>,
        router: Router,
    ) -> &mut Self {
        let Router {
            routes,
            cors: _,
            request_middleware,
            response_middleware,
            shared_extensions,
            not_found_handler,
            typed_error_handlers,
            error_handler,
            debug_errors,
            #[cfg(feature = "validation")]
            validation,
        } = router;
        let prefix = prefix.as_ref();

        self.routes.extend(
            routes
                .into_iter()
                .map(|route| route.with_path_prefix(prefix)),
        );
        self.request_middleware.extend(request_middleware);
        self.response_middleware.extend(response_middleware);
        self.shared_extensions.extend_missing(shared_extensions);
        if let Some(not_found_handler) = not_found_handler {
            self.not_found_handler = Some(not_found_handler);
        }
        self.merge_typed_error_handlers(typed_error_handlers);
        if let Some(error_handler) = error_handler {
            self.error_handler = Some(error_handler);
        }
        if let Some(debug_errors) = debug_errors {
            self.debug_errors = Some(debug_errors);
        }
        #[cfg(feature = "validation")]
        if let Some(validation) = validation {
            self.validation
                .get_or_insert_with(ValidationConfig::new)
                .append(validation);
        }

        self
    }

    /// Returns the most specific route match for a request.
    #[must_use]
    pub fn find(&self, request: &Request) -> Option<RouteMatch<'_>> {
        let mut selected: Option<SelectedRoute<'_>> = None;

        for route in &self.routes {
            let Some(route_match) = route.match_request(request.method(), request.path()) else {
                continue;
            };

            if selected.as_ref().is_none_or(|current| {
                route_takes_precedence(route, route_match.method_score, current)
            }) {
                selected = Some(SelectedRoute {
                    route,
                    path_params: route_match.path_params,
                    method_score: route_match.method_score,
                });
            }
        }

        selected.map(|selected| RouteMatch {
            route: selected.route,
            path_params: selected.path_params,
        })
    }

    /// Handles a request with the matching route or returns `404 Not Found`.
    #[must_use]
    pub fn handle(&self, mut request: Request) -> Response {
        request.set_shared_extensions(self.shared_extensions.clone());
        request = self.apply_request_middleware(request);

        if let Some(cors) = &self.cors {
            if let Some(response) = cors.preflight_response_for_request(&request) {
                return response;
            }
        }

        let Some(route_match) = self.find(&request) else {
            let response = self.not_found_response(&request);
            let response = self.apply_response_middleware(&request, response);
            return self.apply_cors(&request, response);
        };
        let route = route_match.route;

        request.set_matched_route(route.method(), route.path());
        request.set_path_params(&route_match.path_params);
        request = route.apply_request_middleware(request);
        #[cfg(feature = "validation")]
        if let Err(error) = route.validate_request(&request) {
            return self.apply_cors(&request, request_validation_response(&error));
        }
        #[cfg(feature = "validation")]
        if let Some(response) = self.validate_request(&request) {
            return self.apply_cors(&request, response);
        }

        let response = match route.try_handle(&request) {
            Ok(response) => response,
            Err(error) => {
                let response = self.error_response(&request, error.as_ref());
                let response = self.apply_response_middleware(&request, response);
                return self.apply_cors(&request, response);
            }
        };
        let response = route.apply_response_middleware(&request, response);
        let response = self.apply_response_middleware(&request, response);
        #[cfg(feature = "validation")]
        if let Err(error) = route.validate_response(&request, &response) {
            return self.apply_cors(&request, response_validation_response(&error));
        }
        #[cfg(feature = "validation")]
        if let Some(validation_response) = self.validate_response(&request, &response) {
            return self.apply_cors(&request, validation_response);
        }

        self.apply_cors(&request, response)
    }

    /// Returns registered routes in registration order.
    #[must_use]
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// Returns the number of registered routes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Returns true when no routes have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    fn apply_cors(&self, request: &Request, response: Response) -> Response {
        if let Some(cors) = &self.cors {
            cors.apply_for_request(request, response)
        } else {
            response
        }
    }

    fn apply_request_middleware(&self, mut request: Request) -> Request {
        for middleware in &self.request_middleware {
            request = middleware(request);
        }
        request
    }

    fn apply_response_middleware(&self, request: &Request, mut response: Response) -> Response {
        for middleware in &self.response_middleware {
            response = middleware(request, response);
        }
        response
    }

    fn not_found_response(&self, request: &Request) -> Response {
        self.not_found_handler
            .as_ref()
            .map_or_else(Response::not_found, |handler| handler(request))
    }

    fn error_response(&self, request: &Request, error: &RouteError) -> Response {
        self.typed_error_handlers
            .iter()
            .find_map(|entry| entry.handle(request, error))
            .or_else(|| {
                self.error_handler
                    .as_ref()
                    .map(|handler| handler(request, error))
            })
            .unwrap_or_else(|| default_error_response(error, self.debug_errors()))
    }

    fn upsert_typed_error_handler(&mut self, entry: TypedErrorHandlerEntry) {
        self.typed_error_handlers
            .retain(|existing| existing.type_id != entry.type_id);
        self.typed_error_handlers.push(entry);
    }

    fn merge_typed_error_handlers(&mut self, entries: Vec<TypedErrorHandlerEntry>) {
        for entry in entries {
            self.upsert_typed_error_handler(entry);
        }
    }

    #[cfg(feature = "validation")]
    fn validate_request(&self, request: &Request) -> Option<Response> {
        self.validation
            .as_ref()?
            .validate_request(request)
            .err()
            .map(|error| request_validation_response(&error))
    }

    #[cfg(feature = "validation")]
    fn validate_response(&self, request: &Request, response: &Response) -> Option<Response> {
        self.validation
            .as_ref()?
            .validate_response(request, response)
            .err()
            .map(|error| response_validation_response(&error))
    }
}

impl AsyncRouter {
    /// Creates an empty asynchronous router.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            routes: Vec::new(),
            cors: None,
            request_middleware: Vec::new(),
            response_middleware: Vec::new(),
            shared_extensions: Extensions::new(),
            not_found_handler: None,
            typed_error_handlers: Vec::new(),
            error_handler: None,
            debug_errors: None,
            #[cfg(feature = "validation")]
            validation: None,
        }
    }

    /// Enables CORS handling with the provided configuration.
    pub fn enable_cors(&mut self, cors: CorsConfig) -> &mut Self {
        self.cors = Some(cors);
        self
    }

    /// Returns a copy of this router with CORS enabled.
    #[must_use]
    pub fn with_cors(mut self, cors: CorsConfig) -> Self {
        self.cors = Some(cors);
        self
    }

    /// Returns the CORS configuration when enabled.
    #[must_use]
    pub const fn cors(&self) -> Option<&CorsConfig> {
        self.cors.as_ref()
    }

    /// Returns whether default internal-error responses expose error details.
    ///
    /// When not explicitly configured, this follows `POWERTOOLS_DEV`.
    #[must_use]
    pub fn debug_errors(&self) -> bool {
        self.debug_errors.unwrap_or_else(powertools_dev_enabled)
    }

    /// Sets whether default internal-error responses expose error details.
    ///
    /// This only affects unhandled fallible route errors. Built-in
    /// [`HttpError`] responses and custom error handlers keep their existing
    /// response bodies.
    pub fn set_debug_errors(&mut self, enabled: bool) -> &mut Self {
        self.debug_errors = Some(enabled);
        self
    }

    /// Returns a copy of this asynchronous router with default error detail rendering configured.
    #[must_use]
    pub fn with_debug_errors(mut self, enabled: bool) -> Self {
        self.set_debug_errors(enabled);
        self
    }

    /// Returns router shared extension values.
    #[must_use]
    pub const fn shared_extensions(&self) -> &Extensions {
        &self.shared_extensions
    }

    /// Returns a router shared extension value by type.
    #[must_use]
    pub fn shared_extension<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        self.shared_extensions.get::<T>()
    }

    /// Adds or replaces a router shared extension value.
    pub fn insert_shared_extension<T>(&mut self, value: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.shared_extensions.insert(value);
        self
    }

    /// Returns a copy of this router with a router shared extension value.
    #[must_use]
    pub fn with_shared_extension<T>(mut self, value: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        self.insert_shared_extension(value);
        self
    }

    /// Removes all router shared extension values.
    pub fn clear_shared_extensions(&mut self) -> &mut Self {
        self.shared_extensions.clear();
        self
    }

    /// Adds request middleware that runs before CORS preflight handling and route matching.
    pub fn add_request_middleware(
        &mut self,
        middleware: impl Fn(Request) -> Request + Send + Sync + 'static,
    ) -> &mut Self {
        self.request_middleware.push(Box::new(middleware));
        self
    }

    /// Adds response middleware that runs after route handling and before CORS headers are applied.
    pub fn add_response_middleware(
        &mut self,
        middleware: impl Fn(&Request, Response) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.response_middleware.push(Box::new(middleware));
        self
    }

    /// Adds request and response middleware that records per-request HTTP metrics.
    #[cfg(feature = "metrics")]
    pub fn add_metrics_middleware(
        &mut self,
        metrics: std::sync::Arc<std::sync::Mutex<aws_lambda_powertools_metrics::Metrics>>,
    ) -> &mut Self {
        self.add_request_middleware(crate::http_metrics_start_middleware());
        self.add_response_middleware(crate::http_metrics_response_middleware(metrics));
        self
    }

    /// Adds response middleware that records per-request HTTP trace segments.
    #[cfg(feature = "tracer")]
    pub fn add_trace_middleware(
        &mut self,
        tracer: aws_lambda_powertools_tracer::Tracer,
        sink: std::sync::Arc<crate::HttpTraceSink>,
    ) -> &mut Self {
        self.add_response_middleware(crate::http_trace_response_middleware(tracer, sink));
        self
    }

    /// Returns the number of registered request middleware functions.
    #[must_use]
    pub fn request_middleware_len(&self) -> usize {
        self.request_middleware.len()
    }

    /// Returns the number of registered response middleware functions.
    #[must_use]
    pub fn response_middleware_len(&self) -> usize {
        self.response_middleware.len()
    }

    /// Sets the asynchronous handler used when no route matches a request.
    ///
    /// Response middleware and CORS handling still run for custom not-found
    /// responses.
    pub fn set_not_found_handler(
        &mut self,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.not_found_handler = Some(Box::new(handler));
        self
    }

    /// Returns a copy of this asynchronous router with a custom not-found handler.
    #[must_use]
    pub fn with_not_found_handler(
        mut self,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> Self {
        self.set_not_found_handler(handler);
        self
    }

    /// Sets the asynchronous handler used when a fallible route returns an error.
    ///
    /// This handler is only used for routes registered with
    /// [`AsyncRouter::add_fallible_route`] or the `*_fallible` route helpers.
    /// It does not catch panics. Response middleware and CORS handling still
    /// run for custom error responses.
    pub fn set_error_handler(
        &mut self,
        handler: impl for<'a> Fn(&'a Request, &'a RouteError) -> ResponseFuture<'a>
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.error_handler = Some(Box::new(handler));
        self
    }

    /// Sets the asynchronous handler used when a fallible route returns a concrete error type.
    ///
    /// Typed error handlers run before the catch-all handler configured with
    /// [`AsyncRouter::set_error_handler`]. Registering another handler for the
    /// same error type replaces the previous handler.
    pub fn set_error_handler_for<E>(
        &mut self,
        handler: impl for<'a> Fn(&'a Request, &'a E) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.upsert_typed_error_handler(AsyncTypedErrorHandlerEntry::new(handler));
        self
    }

    /// Returns a copy of this asynchronous router with a fallible route error handler.
    #[must_use]
    pub fn with_error_handler(
        mut self,
        handler: impl for<'a> Fn(&'a Request, &'a RouteError) -> ResponseFuture<'a>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.set_error_handler(handler);
        self
    }

    /// Returns a copy of this asynchronous router with a concrete fallible route error handler.
    #[must_use]
    pub fn with_error_handler_for<E>(
        mut self,
        handler: impl for<'a> Fn(&'a Request, &'a E) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.set_error_handler_for(handler);
        self
    }

    /// Enables validation with the provided configuration.
    #[cfg(feature = "validation")]
    pub fn enable_validation(&mut self, validation: ValidationConfig) -> &mut Self {
        self.validation = Some(validation);
        self
    }

    /// Returns a copy of this router with validation enabled.
    #[cfg(feature = "validation")]
    #[must_use]
    pub fn with_validation(mut self, validation: ValidationConfig) -> Self {
        self.validation = Some(validation);
        self
    }

    /// Returns the validation configuration when enabled.
    #[cfg(feature = "validation")]
    #[must_use]
    pub const fn validation(&self) -> Option<&ValidationConfig> {
        self.validation.as_ref()
    }

    /// Adds a request validator, enabling validation when needed.
    #[cfg(feature = "validation")]
    pub fn add_request_validator(
        &mut self,
        validator: impl Fn(&Request) -> aws_lambda_powertools_validation::ValidationResult
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.validation
            .get_or_insert_with(ValidationConfig::new)
            .add_request_validator(validator);
        self
    }

    /// Adds a response validator, enabling validation when needed.
    #[cfg(feature = "validation")]
    pub fn add_response_validator(
        &mut self,
        validator: impl Fn(&Request, &Response) -> aws_lambda_powertools_validation::ValidationResult
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.validation
            .get_or_insert_with(ValidationConfig::new)
            .add_response_validator(validator);
        self
    }

    /// Adds an asynchronous route handler.
    pub fn add_route(
        &mut self,
        method: Method,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.routes.push(AsyncRoute::new(method, path, handler));
        self
    }

    /// Adds the same asynchronous route handler for multiple request methods.
    ///
    /// This is a convenience for registering methods such as `GET` and `POST`
    /// on the same path while preserving normal route precedence.
    pub fn add_routes(
        &mut self,
        methods: impl IntoIterator<Item = Method>,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        let path = path.into();
        let handler: Arc<AsyncHandler> = Arc::new(handler);

        for method in methods {
            self.routes.push(AsyncRoute::new_with_handler(
                method,
                path.clone(),
                Arc::clone(&handler),
            ));
        }

        self
    }

    /// Adds an asynchronous fallible route handler.
    ///
    /// Returned errors are mapped through this router's error handler when one
    /// is configured, otherwise they produce `500 Internal Server Error`.
    pub fn add_fallible_route(
        &mut self,
        method: Method,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.routes
            .push(AsyncRoute::new_fallible(method, path, handler));
        self
    }

    /// Adds the same asynchronous fallible route handler for multiple request methods.
    ///
    /// Returned errors are mapped through this router's error handler when one
    /// is configured, otherwise built-in [`HttpError`] values produce their
    /// status code and other errors produce `500 Internal Server Error`.
    pub fn add_fallible_routes(
        &mut self,
        methods: impl IntoIterator<Item = Method>,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        let path = path.into();
        let handler: Arc<AsyncFallibleHandler> = Arc::new(handler);

        for method in methods {
            self.routes.push(AsyncRoute::new_with_fallible_handler(
                method,
                path.clone(),
                Arc::clone(&handler),
            ));
        }

        self
    }

    /// Registers a prebuilt asynchronous route.
    ///
    /// Use this when the route owns route-specific middleware.
    pub fn register_route(&mut self, route: AsyncRoute) -> &mut Self {
        self.routes.push(route);
        self
    }

    /// Adds an asynchronous `GET` route handler.
    pub fn get(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Get, path, handler)
    }

    /// Adds an asynchronous fallible `GET` route handler.
    pub fn get_fallible(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_fallible_route(Method::Get, path, handler)
    }

    /// Adds an asynchronous `HEAD` route handler.
    pub fn head(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Head, path, handler)
    }

    /// Adds an asynchronous fallible `HEAD` route handler.
    pub fn head_fallible(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_fallible_route(Method::Head, path, handler)
    }

    /// Adds an asynchronous `POST` route handler.
    pub fn post(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Post, path, handler)
    }

    /// Adds an asynchronous fallible `POST` route handler.
    pub fn post_fallible(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_fallible_route(Method::Post, path, handler)
    }

    /// Adds an asynchronous `PUT` route handler.
    pub fn put(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Put, path, handler)
    }

    /// Adds an asynchronous fallible `PUT` route handler.
    pub fn put_fallible(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_fallible_route(Method::Put, path, handler)
    }

    /// Adds an asynchronous `PATCH` route handler.
    pub fn patch(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Patch, path, handler)
    }

    /// Adds an asynchronous fallible `PATCH` route handler.
    pub fn patch_fallible(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_fallible_route(Method::Patch, path, handler)
    }

    /// Adds an asynchronous `DELETE` route handler.
    pub fn delete(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Delete, path, handler)
    }

    /// Adds an asynchronous fallible `DELETE` route handler.
    pub fn delete_fallible(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_fallible_route(Method::Delete, path, handler)
    }

    /// Adds an asynchronous `OPTIONS` route handler.
    pub fn options(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Options, path, handler)
    }

    /// Adds an asynchronous fallible `OPTIONS` route handler.
    pub fn options_fallible(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_fallible_route(Method::Options, path, handler)
    }

    /// Adds an asynchronous route handler that accepts any request method.
    pub fn any(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Any, path, handler)
    }

    /// Adds an asynchronous fallible route handler that accepts any request method.
    pub fn any_fallible(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_fallible_route(Method::Any, path, handler)
    }

    /// Includes routes, middleware, validation hooks, and error handling from another asynchronous router.
    ///
    /// Included routes behave as if they were registered after this router's
    /// existing routes. The included router's CORS configuration is not merged.
    /// If the included router has a not-found or fallible route error handler,
    /// it replaces this router's matching handler.
    pub fn include_router(&mut self, router: AsyncRouter) -> &mut Self {
        self.include_router_with_prefix("", router)
    }

    /// Includes another asynchronous router, prefixing each included route path.
    ///
    /// A prefix of `/api` turns an included route `/orders/{id}` into
    /// `/api/orders/{id}`. Included middleware runs after this router's
    /// existing middleware. The included router's CORS configuration is not
    /// merged.
    pub fn include_router_with_prefix(
        &mut self,
        prefix: impl AsRef<str>,
        router: AsyncRouter,
    ) -> &mut Self {
        let AsyncRouter {
            routes,
            cors: _,
            request_middleware,
            response_middleware,
            shared_extensions,
            not_found_handler,
            typed_error_handlers,
            error_handler,
            debug_errors,
            #[cfg(feature = "validation")]
            validation,
        } = router;
        let prefix = prefix.as_ref();

        self.routes.extend(
            routes
                .into_iter()
                .map(|route| route.with_path_prefix(prefix)),
        );
        self.request_middleware.extend(request_middleware);
        self.response_middleware.extend(response_middleware);
        self.shared_extensions.extend_missing(shared_extensions);
        if let Some(not_found_handler) = not_found_handler {
            self.not_found_handler = Some(not_found_handler);
        }
        self.merge_typed_error_handlers(typed_error_handlers);
        if let Some(error_handler) = error_handler {
            self.error_handler = Some(error_handler);
        }
        if let Some(debug_errors) = debug_errors {
            self.debug_errors = Some(debug_errors);
        }
        #[cfg(feature = "validation")]
        if let Some(validation) = validation {
            self.validation
                .get_or_insert_with(ValidationConfig::new)
                .append(validation);
        }

        self
    }

    /// Returns the most specific route match for a request.
    #[must_use]
    pub fn find(&self, request: &Request) -> Option<AsyncRouteMatch<'_>> {
        let mut selected: Option<SelectedAsyncRoute<'_>> = None;

        for route in &self.routes {
            let Some(route_match) = route.match_request(request.method(), request.path()) else {
                continue;
            };

            if selected.as_ref().is_none_or(|current| {
                async_route_takes_precedence(route, route_match.method_score, current)
            }) {
                selected = Some(SelectedAsyncRoute {
                    route,
                    path_params: route_match.path_params,
                    method_score: route_match.method_score,
                });
            }
        }

        selected.map(|selected| AsyncRouteMatch {
            route: selected.route,
            path_params: selected.path_params,
        })
    }

    /// Handles a request asynchronously with the matching route or returns `404 Not Found`.
    pub async fn handle(&self, mut request: Request) -> Response {
        request.set_shared_extensions(self.shared_extensions.clone());
        request = self.apply_request_middleware(request);

        if let Some(cors) = &self.cors {
            if let Some(response) = cors.preflight_response_for_request(&request) {
                return response;
            }
        }

        let Some(route_match) = self.find(&request) else {
            let response = self.not_found_response(&request).await;
            let response = self.apply_response_middleware(&request, response);
            return self.apply_cors(&request, response);
        };
        let route = route_match.route;

        request.set_matched_route(route.method(), route.path());
        request.set_path_params(&route_match.path_params);
        request = route.apply_request_middleware(request);
        #[cfg(feature = "validation")]
        if let Err(error) = route.validate_request(&request) {
            return self.apply_cors(&request, request_validation_response(&error));
        }
        #[cfg(feature = "validation")]
        if let Some(response) = self.validate_request(&request) {
            return self.apply_cors(&request, response);
        }

        let response = match route.try_handle(&request).await {
            Ok(response) => response,
            Err(error) => {
                let response = self.error_response(&request, error.as_ref()).await;
                let response = self.apply_response_middleware(&request, response);
                return self.apply_cors(&request, response);
            }
        };
        let response = route.apply_response_middleware(&request, response);
        let response = self.apply_response_middleware(&request, response);
        #[cfg(feature = "validation")]
        if let Err(error) = route.validate_response(&request, &response) {
            return self.apply_cors(&request, response_validation_response(&error));
        }
        #[cfg(feature = "validation")]
        if let Some(validation_response) = self.validate_response(&request, &response) {
            return self.apply_cors(&request, validation_response);
        }

        self.apply_cors(&request, response)
    }

    /// Returns registered routes in registration order.
    #[must_use]
    pub fn routes(&self) -> &[AsyncRoute] {
        &self.routes
    }

    /// Returns the number of registered routes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Returns true when no routes have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    fn apply_cors(&self, request: &Request, response: Response) -> Response {
        if let Some(cors) = &self.cors {
            cors.apply_for_request(request, response)
        } else {
            response
        }
    }

    fn apply_request_middleware(&self, mut request: Request) -> Request {
        for middleware in &self.request_middleware {
            request = middleware(request);
        }
        request
    }

    fn apply_response_middleware(&self, request: &Request, mut response: Response) -> Response {
        for middleware in &self.response_middleware {
            response = middleware(request, response);
        }
        response
    }

    fn not_found_response<'a>(&'a self, request: &'a Request) -> ResponseFuture<'a> {
        if let Some(handler) = &self.not_found_handler {
            handler(request)
        } else {
            Box::pin(async { Response::not_found() })
        }
    }

    fn error_response<'a>(
        &'a self,
        request: &'a Request,
        error: &'a RouteError,
    ) -> ResponseFuture<'a> {
        Box::pin(async move {
            for entry in &self.typed_error_handlers {
                if let Some(response) = entry.handle(request, error).await {
                    return response;
                }
            }

            if let Some(handler) = &self.error_handler {
                handler(request, error).await
            } else {
                default_error_response(error, self.debug_errors())
            }
        })
    }

    fn upsert_typed_error_handler(&mut self, entry: AsyncTypedErrorHandlerEntry) {
        self.typed_error_handlers
            .retain(|existing| existing.type_id != entry.type_id);
        self.typed_error_handlers.push(entry);
    }

    fn merge_typed_error_handlers(&mut self, entries: Vec<AsyncTypedErrorHandlerEntry>) {
        for entry in entries {
            self.upsert_typed_error_handler(entry);
        }
    }

    #[cfg(feature = "validation")]
    fn validate_request(&self, request: &Request) -> Option<Response> {
        self.validation
            .as_ref()?
            .validate_request(request)
            .err()
            .map(|error| request_validation_response(&error))
    }

    #[cfg(feature = "validation")]
    fn validate_response(&self, request: &Request, response: &Response) -> Option<Response> {
        self.validation
            .as_ref()?
            .validate_response(request, response)
            .err()
            .map(|error| response_validation_response(&error))
    }
}

impl fmt::Debug for Router {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("Router");
        debug
            .field("routes", &self.routes)
            .field("cors", &self.cors)
            .field("request_middleware_len", &self.request_middleware.len())
            .field("response_middleware_len", &self.response_middleware.len())
            .field("shared_extensions_len", &self.shared_extensions.len())
            .field("has_not_found_handler", &self.not_found_handler.is_some())
            .field("typed_error_handlers_len", &self.typed_error_handlers.len())
            .field("has_error_handler", &self.error_handler.is_some())
            .field("debug_errors", &self.debug_errors());
        #[cfg(feature = "validation")]
        debug.field("validation_enabled", &self.validation.is_some());
        debug.finish()
    }
}

impl fmt::Debug for AsyncRouter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("AsyncRouter");
        debug
            .field("routes", &self.routes)
            .field("cors", &self.cors)
            .field("request_middleware_len", &self.request_middleware.len())
            .field("response_middleware_len", &self.response_middleware.len())
            .field("shared_extensions_len", &self.shared_extensions.len())
            .field("has_not_found_handler", &self.not_found_handler.is_some())
            .field("typed_error_handlers_len", &self.typed_error_handlers.len())
            .field("has_error_handler", &self.error_handler.is_some())
            .field("debug_errors", &self.debug_errors());
        #[cfg(feature = "validation")]
        debug.field("validation_enabled", &self.validation.is_some());
        debug.finish()
    }
}

/// A matched route and the path parameters captured from the request path.
#[derive(Debug)]
pub struct RouteMatch<'a> {
    route: &'a Route,
    path_params: crate::PathParams,
}

impl<'a> RouteMatch<'a> {
    /// Returns the matched route.
    #[must_use]
    pub const fn route(&self) -> &'a Route {
        self.route
    }

    /// Returns captured path parameters.
    #[must_use]
    pub const fn path_params(&self) -> &crate::PathParams {
        &self.path_params
    }

    /// Returns a captured path parameter by name.
    #[must_use]
    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.path_params.get(name)
    }

    /// Consumes the match and returns captured path parameters.
    #[must_use]
    pub fn into_path_params(self) -> crate::PathParams {
        self.path_params
    }
}

/// A matched asynchronous route and the path parameters captured from the request path.
#[derive(Debug)]
pub struct AsyncRouteMatch<'a> {
    route: &'a AsyncRoute,
    path_params: crate::PathParams,
}

impl<'a> AsyncRouteMatch<'a> {
    /// Returns the matched route.
    #[must_use]
    pub const fn route(&self) -> &'a AsyncRoute {
        self.route
    }

    /// Returns captured path parameters.
    #[must_use]
    pub const fn path_params(&self) -> &crate::PathParams {
        &self.path_params
    }

    /// Returns a captured path parameter by name.
    #[must_use]
    pub fn path_param(&self, name: &str) -> Option<&str> {
        self.path_params.get(name)
    }

    /// Consumes the match and returns captured path parameters.
    #[must_use]
    pub fn into_path_params(self) -> crate::PathParams {
        self.path_params
    }
}

struct SelectedRoute<'a> {
    route: &'a Route,
    path_params: crate::PathParams,
    method_score: u8,
}

struct SelectedAsyncRoute<'a> {
    route: &'a AsyncRoute,
    path_params: crate::PathParams,
    method_score: u8,
}

struct TypedErrorHandlerEntry {
    type_id: TypeId,
    handler: Box<TypedErrorHandler>,
}

impl TypedErrorHandlerEntry {
    fn new<E>(handler: impl Fn(&Request, &E) -> Response + Send + Sync + 'static) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            type_id: TypeId::of::<E>(),
            handler: Box::new(move |request, error| {
                error
                    .downcast_ref::<E>()
                    .map(|error| handler(request, error))
            }),
        }
    }

    fn handle(&self, request: &Request, error: &RouteError) -> Option<Response> {
        (self.handler)(request, error)
    }
}

struct AsyncTypedErrorHandlerEntry {
    type_id: TypeId,
    handler: Box<AsyncTypedErrorHandler>,
}

impl AsyncTypedErrorHandlerEntry {
    fn new<E>(
        handler: impl for<'a> Fn(&'a Request, &'a E) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            type_id: TypeId::of::<E>(),
            handler: Box::new(move |request, error| {
                if let Some(error) = error.downcast_ref::<E>() {
                    let response = handler(request, error);
                    Box::pin(async move { Some(response.await) })
                } else {
                    Box::pin(async { None })
                }
            }),
        }
    }

    fn handle<'a>(
        &'a self,
        request: &'a Request,
        error: &'a RouteError,
    ) -> OptionalResponseFuture<'a> {
        (self.handler)(request, error)
    }
}

fn route_takes_precedence(
    candidate: &Route,
    candidate_method_score: u8,
    selected: &SelectedRoute<'_>,
) -> bool {
    match candidate
        .path_precedence()
        .cmp(selected.route.path_precedence())
    {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => candidate_method_score > selected.method_score,
    }
}

fn async_route_takes_precedence(
    candidate: &AsyncRoute,
    candidate_method_score: u8,
    selected: &SelectedAsyncRoute<'_>,
) -> bool {
    match candidate
        .path_precedence()
        .cmp(selected.route.path_precedence())
    {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => candidate_method_score > selected.method_score,
    }
}

fn default_error_response(error: &RouteError, debug_errors: bool) -> Response {
    if let Some(error) = error.downcast_ref::<HttpError>() {
        return error.to_response();
    }

    if debug_errors {
        Response::internal_server_error().with_body(error.to_string())
    } else {
        Response::internal_server_error()
    }
}

fn powertools_dev_enabled() -> bool {
    debug_errors_from_source(env::var)
}

fn debug_errors_from_source(mut source: impl FnMut(&str) -> Option<String>) -> bool {
    source(env::POWERTOOLS_DEV).is_some_and(|value| env::is_truthy(&value))
}

#[cfg(test)]
mod tests {
    use std::{fmt, future::Future};

    use aws_lambda_powertools_core::env;
    #[cfg(feature = "validation")]
    use aws_lambda_powertools_validation::Validator;
    use futures_executor::block_on;

    #[cfg(feature = "validation")]
    use crate::ValidationConfig;
    use crate::{
        AsyncRoute, AsyncRouter, CorsConfig, FallibleResponseFuture, HttpError, Method, Request,
        Response, ResponseFuture, Route, RouteError, RouteResult, Router,
    };

    #[derive(Debug)]
    struct TestRouteError(&'static str);

    impl fmt::Display for TestRouteError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for TestRouteError {}

    #[derive(Debug)]
    struct OtherRouteError(&'static str);

    impl fmt::Display for OtherRouteError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for OtherRouteError {}

    #[derive(Debug, Eq, PartialEq)]
    struct CorrelationId(String);

    #[derive(Debug, Eq, PartialEq)]
    struct ServiceName(&'static str);

    fn async_response<'a>(
        future: impl Future<Output = Response> + Send + 'a,
    ) -> ResponseFuture<'a> {
        Box::pin(future)
    }

    fn async_fallible_response<'a>(
        future: impl Future<Output = RouteResult> + Send + 'a,
    ) -> FallibleResponseFuture<'a> {
        Box::pin(future)
    }

    #[test]
    fn static_route_precedes_dynamic_peer_even_when_registered_later() {
        let mut router = Router::new();
        router.get("/orders/{id}", |request| {
            Response::ok(format!(
                "dynamic:{}",
                request.path_param("id").expect("id is captured")
            ))
        });
        router.get("/orders/list", |_| Response::ok("static"));

        let response = router.handle(Request::new(Method::Get, "/orders/list"));

        assert_eq!(response.body(), b"static");
    }

    #[test]
    fn static_path_precedence_applies_before_any_method_precedence() {
        let mut router = Router::new();
        router.get("/orders/{id}", |_| Response::ok("get dynamic"));
        router.any("/orders/list", |_| Response::ok("any static"));

        let response = router.handle(Request::new(Method::Get, "/orders/list"));

        assert_eq!(response.body(), b"any static");
    }

    #[test]
    fn exact_method_route_precedes_any_route_for_same_path() {
        let mut router = Router::new();
        router.any("/health", |_| Response::ok("any"));
        router.get("/health", |_| Response::ok("get"));

        let get_response = router.handle(Request::new(Method::Get, "/health"));
        let post_response = router.handle(Request::new(Method::Post, "/health"));

        assert_eq!(get_response.body(), b"get");
        assert_eq!(post_response.body(), b"any");
    }

    #[test]
    fn add_routes_registers_same_handler_for_multiple_methods() {
        let mut router = Router::new();
        router.add_routes([Method::Get, Method::Post], "/orders", |request| {
            Response::ok(request.method().as_str())
        });

        let get_response = router.handle(Request::new(Method::Get, "/orders"));
        let post_response = router.handle(Request::new(Method::Post, "/orders"));
        let delete_response = router.handle(Request::new(Method::Delete, "/orders"));

        assert_eq!(router.routes().len(), 2);
        assert_eq!(get_response.body(), b"GET");
        assert_eq!(post_response.body(), b"POST");
        assert_eq!(delete_response.status_code(), 404);
    }

    #[test]
    fn add_fallible_routes_registers_same_handler_for_multiple_methods() {
        let mut router = Router::new();
        router.add_fallible_routes([Method::Post, Method::Put], "/orders", |request| {
            Err(HttpError::bad_request(format!(
                "invalid {}",
                request.method()
            )))
        });

        let post_response = router.handle(Request::new(Method::Post, "/orders"));
        let put_response = router.handle(Request::new(Method::Put, "/orders"));
        let get_response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(router.routes().len(), 2);
        assert_eq!(post_response.status_code(), 400);
        assert_eq!(post_response.body(), b"invalid POST");
        assert_eq!(put_response.status_code(), 400);
        assert_eq!(put_response.body(), b"invalid PUT");
        assert_eq!(get_response.status_code(), 404);
    }

    #[test]
    fn dynamic_route_captures_path_params() {
        let mut router = Router::new();
        router.get("/orders/{order_id}/items/{item_id}", |request| {
            Response::ok(format!(
                "{}:{}",
                request
                    .path_param("order_id")
                    .expect("order_id is captured"),
                request.path_param("item_id").expect("item_id is captured")
            ))
        });

        let request = Request::new(Method::Get, "/orders/order-1/items/item-7");
        let route_match = router.find(&request).expect("route matches");
        let response = router.handle(request);

        assert_eq!(route_match.path_param("order_id"), Some("order-1"));
        assert_eq!(route_match.path_param("item_id"), Some("item-7"));
        assert_eq!(response.body(), b"order-1:item-7");
    }

    #[test]
    fn matched_route_is_available_to_handlers_and_response_middleware() {
        let mut router = Router::new();
        router.add_response_middleware(|request, response| {
            let route = request
                .matched_route()
                .map_or_else(|| "NOT_FOUND".to_owned(), crate::MatchedRoute::label);
            response.with_header("x-route", route)
        });
        router.get("/orders/{order_id}", |request| {
            let route = request.matched_route().expect("route is matched");

            assert_eq!(route.method(), Method::Get);
            assert_eq!(route.path(), "/orders/{order_id}");

            Response::ok(route.label())
        });

        let response = router.handle(Request::new(Method::Get, "/orders/order-1"));

        assert_eq!(response.body(), b"GET /orders/{order_id}");
        assert_eq!(response.header("x-route"), Some("GET /orders/{order_id}"));
    }

    #[test]
    fn unmatched_requests_do_not_have_matched_route_metadata() {
        let mut router = Router::new();
        router.add_response_middleware(|request, response| {
            assert!(request.matched_route().is_none());
            response.with_header("x-route", "NOT_FOUND")
        });

        let response = router.handle(Request::new(Method::Get, "/missing"));

        assert_eq!(response.status_code(), 404);
        assert_eq!(response.header("x-route"), Some("NOT_FOUND"));
    }

    #[test]
    fn unmatched_request_returns_not_found() {
        let router = Router::new();

        let response = router.handle(Request::new(Method::Get, "/missing"));

        assert_eq!(response.status_code(), 404);
        assert_eq!(response.body(), b"Not Found");
    }

    #[test]
    fn custom_not_found_handler_runs_response_middleware() {
        let mut router = Router::new();
        router.set_not_found_handler(|request| {
            Response::new(410).with_body(format!("missing:{}", request.path()))
        });
        router.add_response_middleware(|_, response| response.with_header("x-middleware", "1"));

        let response = router.handle(Request::new(Method::Get, "/missing"));

        assert_eq!(response.status_code(), 410);
        assert_eq!(response.body(), b"missing:/missing");
        assert_eq!(response.header("x-middleware"), Some("1"));
    }

    #[test]
    fn fallible_route_errors_use_custom_error_handler_and_response_middleware() {
        let mut router = Router::new().with_cors(CorsConfig::default());
        router.set_error_handler(|request, error| {
            Response::new(409).with_body(format!("{}:{}", request.path(), error))
        });
        router.add_response_middleware(|_, response| response.with_header("x-middleware", "1"));
        router.get_fallible("/orders", |_| Err(TestRouteError("duplicate order")));

        let response = router.handle(
            Request::new(Method::Get, "/orders").with_header("Origin", "https://example.com"),
        );

        assert_eq!(response.status_code(), 409);
        assert_eq!(response.body(), b"/orders:duplicate order");
        assert_eq!(response.header("x-middleware"), Some("1"));
        assert_eq!(response.header("access-control-allow-origin"), Some("*"));
    }

    #[test]
    fn typed_error_handler_runs_before_catch_all_error_handler() {
        let mut router = Router::new().with_cors(CorsConfig::default());
        router.set_error_handler(|_, _| Response::internal_server_error());
        router.set_error_handler_for::<TestRouteError>(|request, error| {
            Response::new(409).with_body(format!("{}:{}", request.path(), error))
        });
        router.add_response_middleware(|_, response| response.with_header("x-middleware", "1"));
        router.get_fallible("/orders", |_| Err(TestRouteError("duplicate order")));

        let response = router.handle(
            Request::new(Method::Get, "/orders").with_header("Origin", "https://example.com"),
        );

        assert_eq!(response.status_code(), 409);
        assert_eq!(response.body(), b"/orders:duplicate order");
        assert_eq!(response.header("x-middleware"), Some("1"));
        assert_eq!(response.header("access-control-allow-origin"), Some("*"));
    }

    #[test]
    fn unmatched_typed_error_handler_falls_back_to_catch_all_error_handler() {
        let mut router = Router::new();
        router.set_error_handler_for::<OtherRouteError>(|_, _| Response::new(418));
        router.set_error_handler(|_, error| Response::new(422).with_body(error.to_string()));
        router.get_fallible("/orders", |_| Err(TestRouteError("invalid order")));

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(response.status_code(), 422);
        assert_eq!(response.body(), b"invalid order");
    }

    #[test]
    fn fallible_route_errors_default_to_internal_server_error() {
        let mut router = Router::new().with_debug_errors(false);
        router.post_fallible("/orders", |_| Err(TestRouteError("hidden detail")));

        let response = router.handle(Request::new(Method::Post, "/orders"));

        assert_eq!(response.status_code(), 500);
        assert_eq!(response.body(), b"Internal Server Error");
    }

    #[test]
    fn fallible_route_errors_can_expose_details_in_debug_mode() {
        let mut router = Router::new().with_debug_errors(true);
        router.post_fallible("/orders", |_| Err(TestRouteError("debug detail")));

        let response = router.handle(Request::new(Method::Post, "/orders"));

        assert_eq!(response.status_code(), 500);
        assert_eq!(response.body(), b"debug detail");
    }

    #[test]
    fn debug_error_defaults_follow_powertools_dev() {
        assert!(super::debug_errors_from_source(|name| {
            (name == env::POWERTOOLS_DEV).then(|| "true".to_owned())
        }));
        assert!(!super::debug_errors_from_source(|name| {
            (name == env::POWERTOOLS_DEV).then(|| "false".to_owned())
        }));
        assert!(!super::debug_errors_from_source(|name| {
            (name == env::POWERTOOLS_DEV).then(|| "maybe".to_owned())
        }));
    }

    #[test]
    fn fallible_http_errors_map_to_status_responses_by_default() {
        let mut router = Router::new();
        router.get_fallible("/orders/{id}", |request| {
            Err(HttpError::not_found(format!(
                "missing {}",
                request.path_param("id").unwrap_or_default()
            )))
        });

        let response = router.handle(Request::new(Method::Get, "/orders/order-1"));

        assert_eq!(response.status_code(), 404);
        assert_eq!(response.body(), b"missing order-1");
    }

    #[test]
    fn cors_headers_are_added_to_routed_and_not_found_responses() {
        let mut router = Router::new().with_cors(CorsConfig::new("https://example.com"));
        router.get("/orders", |_| Response::ok("orders"));

        let orders_response = router.handle(
            Request::new(Method::Get, "/orders").with_header("Origin", "https://example.com"),
        );
        let not_found = router.handle(
            Request::new(Method::Get, "/missing").with_header("Origin", "https://example.com"),
        );

        assert_eq!(
            orders_response.header("access-control-allow-origin"),
            Some("https://example.com")
        );
        assert_eq!(
            not_found.header("access-control-allow-origin"),
            Some("https://example.com")
        );
        assert_eq!(not_found.status_code(), 404);
    }

    #[test]
    fn cors_preflight_request_returns_no_content_without_route() {
        let router = Router::new().with_cors(CorsConfig::default());
        let request = Request::new(Method::Options, "/orders")
            .with_header("Origin", "https://example.com")
            .with_header("Access-Control-Request-Method", "POST");

        let response = router.handle(request);

        assert_eq!(response.status_code(), 204);
        assert_eq!(response.header("access-control-allow-origin"), Some("*"));
        assert_eq!(
            response.header("access-control-allow-methods"),
            Some("GET,HEAD,POST,PUT,PATCH,DELETE,OPTIONS")
        );
    }

    #[test]
    fn enable_cors_updates_router_configuration() {
        let mut router = Router::new();
        router.enable_cors(CorsConfig::new("https://example.com"));

        assert_eq!(
            router.cors().map(CorsConfig::allow_origin),
            Some("https://example.com")
        );
    }

    #[test]
    fn request_middleware_runs_before_route_matching() {
        let mut router = Router::new();
        router.add_request_middleware(|request| {
            if request.path() == "/legacy-orders" {
                Request::new(request.method(), "/orders")
            } else {
                request
            }
        });
        router.get("/orders", |_| Response::ok("orders"));

        let response = router.handle(Request::new(Method::Get, "/legacy-orders"));

        assert_eq!(router.request_middleware_len(), 1);
        assert_eq!(response.body(), b"orders");
    }

    #[test]
    fn response_middleware_runs_before_cors_headers_are_applied() {
        let mut router = Router::new().with_cors(CorsConfig::default());
        router.add_response_middleware(|request, response| {
            response.with_header("x-path", request.path())
        });
        router.get("/orders", |_| Response::ok("orders"));

        let response = router.handle(
            Request::new(Method::Get, "/orders").with_header("Origin", "https://example.com"),
        );

        assert_eq!(router.response_middleware_len(), 1);
        assert_eq!(response.header("x-path"), Some("/orders"));
        assert_eq!(response.header("access-control-allow-origin"), Some("*"));
    }

    #[test]
    fn request_extensions_flow_from_middleware_to_handlers() {
        let mut router = Router::new();
        router.add_request_middleware(|request| {
            request.with_extension(CorrelationId("request-1".to_owned()))
        });
        router.add_response_middleware(|request, response| {
            let request_id = request
                .extension::<CorrelationId>()
                .map_or("missing", |correlation_id| correlation_id.0.as_str());
            response.with_header("x-request-id", request_id)
        });
        router.get("/orders", |request| {
            let request_id = request
                .extension::<CorrelationId>()
                .map_or("missing", |correlation_id| correlation_id.0.as_str());
            Response::ok(request_id)
        });

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(response.body(), b"request-1");
        assert_eq!(response.header("x-request-id"), Some("request-1"));
    }

    #[test]
    fn router_shared_extensions_are_attached_before_middleware() {
        let mut router = Router::new().with_shared_extension(ServiceName("checkout"));
        router.add_request_middleware(|request| {
            let service_name = request
                .shared_extension::<ServiceName>()
                .map_or("missing", |service_name| service_name.0);
            request.with_header("x-service", service_name)
        });
        router.get("/orders", |request| {
            Response::ok(request.header("x-service").unwrap_or("missing"))
        });

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(
            router.shared_extension::<ServiceName>(),
            Some(&ServiceName("checkout"))
        );
        assert_eq!(response.body(), b"checkout");
    }

    #[test]
    fn include_router_merges_child_shared_extensions() {
        let mut child = Router::new().with_shared_extension(ServiceName("orders"));
        child.get("/orders", |request| {
            let service_name = request
                .shared_extension::<ServiceName>()
                .map_or("missing", |service_name| service_name.0);
            Response::ok(service_name)
        });

        let mut router = Router::new();
        router.include_router(child);

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(router.shared_extensions().len(), 1);
        assert_eq!(response.body(), b"orders");
    }

    #[test]
    fn route_specific_middleware_runs_around_handler() {
        let route = Route::new(Method::Get, "/orders/{id}", |request| {
            Response::ok(request.header("x-order-id").unwrap_or_default())
        })
        .with_request_middleware(|request| {
            let order_id = request.path_param("id").unwrap_or_default().to_owned();
            request.with_header("x-order-id", order_id)
        })
        .with_response_middleware(|_, response| response.with_header("x-order", "route"));

        let mut router = Router::new();
        router.add_response_middleware(|_, response| response.with_header("x-order", "router"));
        router.register_route(route);

        let response = router.handle(Request::new(Method::Get, "/orders/order-1"));
        let order_headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| (name == "x-order").then_some(value.as_str()))
            .collect::<Vec<_>>();

        assert_eq!(router.routes()[0].request_middleware_len(), 1);
        assert_eq!(router.routes()[0].response_middleware_len(), 1);
        assert_eq!(response.body(), b"order-1");
        assert_eq!(order_headers, vec!["route", "router"]);
    }

    #[test]
    fn include_router_with_prefix_routes_child_paths() {
        let mut child = Router::new();
        child.get("/orders/{id}", |request| {
            Response::ok(format!(
                "order:{}",
                request.path_param("id").expect("id is captured")
            ))
        });

        let mut router = Router::new();
        router.include_router_with_prefix("/api", child);

        let response = router.handle(Request::new(Method::Get, "/api/orders/order-1"));

        assert_eq!(router.len(), 1);
        assert_eq!(router.routes()[0].path(), "/api/orders/{id}");
        assert_eq!(response.body(), b"order:order-1");
    }

    #[test]
    fn include_router_preserves_route_specific_middleware_with_prefix() {
        let route = Route::new(Method::Get, "/orders/{id}", |request| {
            Response::ok(request.header("x-order-id").unwrap_or_default())
        })
        .with_request_middleware(|request| {
            let order_id = request.path_param("id").unwrap_or_default().to_owned();
            request.with_header("x-order-id", order_id)
        });

        let mut child = Router::new();
        child.register_route(route);

        let mut router = Router::new();
        router.include_router_with_prefix("/api", child);

        let response = router.handle(Request::new(Method::Get, "/api/orders/order-1"));

        assert_eq!(router.routes()[0].path(), "/api/orders/{id}");
        assert_eq!(router.routes()[0].request_middleware_len(), 1);
        assert_eq!(response.body(), b"order-1");
    }

    #[test]
    fn include_router_merges_middleware_after_existing_middleware() {
        let mut child = Router::new();
        child.add_response_middleware(|_, response| response.with_header("x-child", "1"));
        child.get("/orders", |_| Response::ok("orders"));

        let mut router = Router::new();
        router.add_response_middleware(|_, response| response.with_header("x-parent", "1"));
        router.include_router(child);

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(router.response_middleware_len(), 2);
        assert_eq!(response.header("x-parent"), Some("1"));
        assert_eq!(response.header("x-child"), Some("1"));
    }

    #[test]
    fn include_router_merges_not_found_handler() {
        let mut child = Router::new();
        child.set_not_found_handler(|_| Response::new(410).with_body("child missing"));

        let mut router = Router::new().with_not_found_handler(|_| Response::not_found());
        router.include_router(child);

        let response = router.handle(Request::new(Method::Get, "/missing"));

        assert_eq!(response.status_code(), 410);
        assert_eq!(response.body(), b"child missing");
    }

    #[test]
    fn include_router_merges_error_handler() {
        let mut child = Router::new();
        child.set_error_handler(|_, error| Response::new(422).with_body(error.to_string()));
        child.get_fallible("/orders", |_| Err(TestRouteError("invalid order")));

        let mut router = Router::new().with_error_handler(|_, _| Response::internal_server_error());
        router.include_router(child);

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(response.status_code(), 422);
        assert_eq!(response.body(), b"invalid order");
    }

    #[test]
    fn include_router_merges_typed_error_handlers_with_child_precedence() {
        let mut child = Router::new();
        child.set_error_handler_for::<TestRouteError>(|_, error| {
            Response::new(409).with_body(format!("child:{error}"))
        });
        child.get_fallible("/orders", |_| Err(TestRouteError("duplicate order")));

        let mut router = Router::new()
            .with_error_handler_for::<TestRouteError>(|_, _| Response::internal_server_error());
        router.include_router(child);

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(response.status_code(), 409);
        assert_eq!(response.body(), b"child:duplicate order");
    }

    #[cfg(feature = "validation")]
    #[test]
    fn request_validation_runs_after_route_matching_and_before_handler() {
        let mut router = Router::new();
        router.add_request_validator(|request| {
            Validator::new().ensure(
                "order_id",
                request.path_param("order_id") == Some("order-1"),
                "order_id must be order-1",
            )
        });
        router.get("/orders/{order_id}", |_| Response::ok("should not run"));

        let response = router.handle(Request::new(Method::Get, "/orders/order-2"));

        assert_eq!(response.status_code(), 422);
        assert_eq!(response.header("content-type"), Some("text/plain"));
        assert_eq!(
            response.body(),
            b"Request validation failed: order_id must be order-1"
        );
    }

    #[cfg(feature = "validation")]
    #[test]
    fn response_validation_runs_after_response_middleware() {
        let mut router = Router::new().with_validation(
            ValidationConfig::new().with_response_validator(|_, response| {
                Validator::new().ensure("body", response.body() == b"ok", "body must be ok")
            }),
        );
        router.add_response_middleware(|_, response| response.with_body("not-ok"));
        router.get("/orders", |_| Response::ok("ok"));

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(response.status_code(), 500);
        assert_eq!(
            response.body(),
            b"Response validation failed: body must be ok"
        );
    }

    #[cfg(feature = "validation")]
    #[test]
    fn include_router_merges_validation_hooks() {
        let mut child = Router::new();
        child.add_request_validator(|request| {
            Validator::new().ensure(
                "tenant",
                request.header("x-tenant").is_some(),
                "tenant header is required",
            )
        });
        child.get("/orders", |_| Response::ok("orders"));

        let mut router = Router::new();
        router.include_router(child);

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(
            router
                .validation()
                .map(ValidationConfig::request_validators_len),
            Some(1)
        );
        assert_eq!(response.status_code(), 422);
        assert_eq!(
            response.body(),
            b"Request validation failed: tenant header is required"
        );
    }

    #[cfg(feature = "validation")]
    #[test]
    fn route_specific_request_validation_runs_after_route_middleware() {
        let route = Route::new(Method::Get, "/orders/{id}", |_| Response::ok("orders"))
            .with_request_middleware(|request| {
                let order_id = request.path_param("id").unwrap_or_default().to_owned();
                request.with_header("x-order-id", order_id)
            })
            .with_request_validator(|request| {
                Validator::new().ensure(
                    "order_id",
                    request.header("x-order-id") == Some("order-1"),
                    "order header is required",
                )
            });

        let mut router = Router::new();
        router.register_route(route);

        let response = router.handle(Request::new(Method::Get, "/orders/order-1"));

        assert_eq!(
            router.routes()[0]
                .validation()
                .map(ValidationConfig::request_validators_len),
            Some(1)
        );
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.body(), b"orders");
    }

    #[cfg(feature = "validation")]
    #[test]
    fn route_specific_response_validation_runs_after_response_middleware() {
        let route = Route::new(Method::Get, "/orders", |_| Response::ok("orders"))
            .with_response_middleware(|_, response| response.with_body("not-ok"))
            .with_response_validator(|_, response| {
                Validator::new().ensure("body", response.body() == b"orders", "body must be orders")
            });

        let mut router = Router::new();
        router.register_route(route);

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(response.status_code(), 500);
        assert_eq!(
            response.body(),
            b"Response validation failed: body must be orders"
        );
    }

    #[cfg(feature = "validation")]
    #[test]
    fn async_router_runs_request_validation_before_handler() {
        let mut router = AsyncRouter::new();
        router.add_request_validator(|request| {
            Validator::new().required_text_field(
                "body",
                std::str::from_utf8(request.body()).unwrap_or_default(),
            )
        });
        router.post("/orders", |_| {
            async_response(async { Response::ok("created") })
        });

        let response =
            block_on(router.handle(Request::new(Method::Post, "/orders").with_body("  ")));

        assert_eq!(response.status_code(), 422);
        assert_eq!(
            response.body(),
            b"Request validation failed: body is required"
        );
        assert_eq!(
            router
                .validation()
                .map(ValidationConfig::request_validators_len),
            Some(1)
        );
    }

    #[test]
    fn async_router_reads_request_extensions() {
        let mut router = AsyncRouter::new().with_shared_extension(ServiceName("checkout"));
        router.add_request_middleware(|request| {
            let service_name = request
                .shared_extension::<ServiceName>()
                .map_or("missing", |service_name| service_name.0);
            request.with_extension(CorrelationId(format!("{service_name}-request")))
        });
        router.get("/orders", |request| {
            let request_id = request.extension::<CorrelationId>().map_or_else(
                || "missing".to_owned(),
                |correlation_id| correlation_id.0.clone(),
            );
            async_response(async move { Response::ok(request_id) })
        });

        let response = block_on(router.handle(Request::new(Method::Get, "/orders")));

        assert_eq!(response.body(), b"checkout-request");
    }

    #[cfg(feature = "validation")]
    #[test]
    fn async_route_specific_request_validation_runs_after_route_middleware() {
        let route = AsyncRoute::new(Method::Get, "/orders/{id}", |_| {
            async_response(async { Response::ok("orders") })
        })
        .with_request_middleware(|request| {
            let order_id = request.path_param("id").unwrap_or_default().to_owned();
            request.with_header("x-order-id", order_id)
        })
        .with_request_validator(|request| {
            Validator::new().ensure(
                "order_id",
                request.header("x-order-id") == Some("order-1"),
                "order header is required",
            )
        });

        let mut router = AsyncRouter::new();
        router.register_route(route);

        let response = block_on(router.handle(Request::new(Method::Get, "/orders/order-1")));

        assert_eq!(
            router.routes()[0]
                .validation()
                .map(ValidationConfig::request_validators_len),
            Some(1)
        );
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.body(), b"orders");
    }

    #[test]
    fn async_router_handles_dynamic_routes() {
        let mut router = AsyncRouter::new();
        router.get("/orders/{id}", |request| {
            async_response(async move {
                Response::ok(format!(
                    "order:{}",
                    request.path_param("id").expect("id is captured")
                ))
            })
        });

        let request = Request::new(Method::Get, "/orders/order-1");
        let route_match = router.find(&request).expect("route matches");
        let response = block_on(router.handle(request));

        assert_eq!(route_match.path_param("id"), Some("order-1"));
        assert_eq!(response.body(), b"order:order-1");
    }

    #[test]
    fn async_router_exposes_matched_route_metadata() {
        let mut router = AsyncRouter::new();
        router.add_response_middleware(|request, response| {
            let route = request
                .matched_route()
                .map_or_else(|| "NOT_FOUND".to_owned(), crate::MatchedRoute::label);
            response.with_header("x-route", route)
        });
        router.get("/jobs/{id}", |request| {
            async_response(async move {
                let route = request.matched_route().expect("route is matched");

                assert_eq!(route.method(), Method::Get);
                assert_eq!(route.path(), "/jobs/{id}");

                Response::ok(route.label())
            })
        });

        let response = block_on(router.handle(Request::new(Method::Get, "/jobs/job-1")));

        assert_eq!(response.body(), b"GET /jobs/{id}");
        assert_eq!(response.header("x-route"), Some("GET /jobs/{id}"));
    }

    #[test]
    fn async_add_routes_registers_same_handler_for_multiple_methods() {
        let mut router = AsyncRouter::new();
        router.add_routes([Method::Get, Method::Patch], "/jobs", |request| {
            let method = request.method();
            async_response(async move { Response::ok(method.as_str()) })
        });

        let get_response = block_on(router.handle(Request::new(Method::Get, "/jobs")));
        let patch_response = block_on(router.handle(Request::new(Method::Patch, "/jobs")));
        let delete_response = block_on(router.handle(Request::new(Method::Delete, "/jobs")));

        assert_eq!(router.routes().len(), 2);
        assert_eq!(get_response.body(), b"GET");
        assert_eq!(patch_response.body(), b"PATCH");
        assert_eq!(delete_response.status_code(), 404);
    }

    #[test]
    fn async_add_fallible_routes_registers_same_handler_for_multiple_methods() {
        let mut router = AsyncRouter::new();
        router.add_fallible_routes([Method::Post, Method::Put], "/jobs", |request| {
            let method = request.method();
            async_fallible_response(async move {
                Err(
                    Box::new(HttpError::bad_request(format!("invalid {method}")))
                        as Box<RouteError>,
                )
            })
        });

        let post_response = block_on(router.handle(Request::new(Method::Post, "/jobs")));
        let put_response = block_on(router.handle(Request::new(Method::Put, "/jobs")));
        let get_response = block_on(router.handle(Request::new(Method::Get, "/jobs")));

        assert_eq!(router.routes().len(), 2);
        assert_eq!(post_response.status_code(), 400);
        assert_eq!(post_response.body(), b"invalid POST");
        assert_eq!(put_response.status_code(), 400);
        assert_eq!(put_response.body(), b"invalid PUT");
        assert_eq!(get_response.status_code(), 404);
    }

    #[test]
    fn async_custom_not_found_handler_runs_response_middleware() {
        let mut router = AsyncRouter::new();
        router.set_not_found_handler(|request| {
            async_response(async move {
                Response::new(410).with_body(format!("missing:{}", request.path()))
            })
        });
        router.add_response_middleware(|_, response| response.with_header("x-middleware", "1"));

        let response = block_on(router.handle(Request::new(Method::Get, "/missing")));

        assert_eq!(response.status_code(), 410);
        assert_eq!(response.body(), b"missing:/missing");
        assert_eq!(response.header("x-middleware"), Some("1"));
    }

    #[test]
    fn async_fallible_route_errors_use_custom_error_handler() {
        let mut router = AsyncRouter::new();
        router.set_error_handler(|request, error| {
            async_response(async move {
                Response::new(409).with_body(format!("{}:{}", request.path(), error))
            })
        });
        router.get_fallible("/orders", |_| {
            async_fallible_response(async {
                Err(Box::new(TestRouteError("duplicate order")) as Box<crate::RouteError>)
            })
        });

        let response = block_on(router.handle(Request::new(Method::Get, "/orders")));

        assert_eq!(response.status_code(), 409);
        assert_eq!(response.body(), b"/orders:duplicate order");
    }

    #[test]
    fn async_typed_error_handler_runs_before_catch_all_error_handler() {
        let mut router = AsyncRouter::new();
        router
            .set_error_handler(|_, _| async_response(async { Response::internal_server_error() }));
        router.set_error_handler_for::<TestRouteError>(|request, error| {
            let body = format!("{}:{}", request.path(), error);
            async_response(async move { Response::new(409).with_body(body) })
        });
        router.get_fallible("/orders", |_| {
            async_fallible_response(async {
                Err(Box::new(TestRouteError("duplicate order")) as Box<RouteError>)
            })
        });

        let response = block_on(router.handle(Request::new(Method::Get, "/orders")));

        assert_eq!(response.status_code(), 409);
        assert_eq!(response.body(), b"/orders:duplicate order");
    }

    #[test]
    fn async_fallible_route_errors_can_expose_details_in_debug_mode() {
        let mut router = AsyncRouter::new().with_debug_errors(true);
        router.get_fallible("/orders", |_| {
            async_fallible_response(async {
                Err(Box::new(TestRouteError("async debug detail")) as Box<RouteError>)
            })
        });

        let response = block_on(router.handle(Request::new(Method::Get, "/orders")));

        assert_eq!(response.status_code(), 500);
        assert_eq!(response.body(), b"async debug detail");
    }

    #[test]
    fn async_fallible_http_errors_map_to_status_responses_by_default() {
        let mut router = AsyncRouter::new();
        router.get_fallible("/orders", |_| {
            async_fallible_response(async {
                Err(Box::new(HttpError::bad_request("invalid order")) as Box<crate::RouteError>)
            })
        });

        let response = block_on(router.handle(Request::new(Method::Get, "/orders")));

        assert_eq!(response.status_code(), 400);
        assert_eq!(response.body(), b"invalid order");
    }

    #[test]
    fn async_router_runs_middleware_and_cors() {
        let mut router = AsyncRouter::new().with_cors(CorsConfig::default());
        router.add_request_middleware(|request| {
            if request.path() == "/legacy-orders" {
                let mut rewritten = Request::new(request.method(), "/orders");
                if let Some(origin) = request.header("Origin") {
                    rewritten = rewritten.with_header("Origin", origin.to_owned());
                }
                rewritten
            } else {
                request
            }
        });
        router.add_response_middleware(|request, response| {
            response.with_header("x-path", request.path())
        });
        router.get("/orders", |request| {
            async_response(async move { Response::ok(request.path().to_owned()) })
        });

        let response = block_on(
            router.handle(
                Request::new(Method::Get, "/legacy-orders")
                    .with_header("Origin", "https://example.com"),
            ),
        );

        assert_eq!(router.request_middleware_len(), 1);
        assert_eq!(router.response_middleware_len(), 1);
        assert_eq!(response.body(), b"/orders");
        assert_eq!(response.header("x-path"), Some("/orders"));
        assert_eq!(response.header("access-control-allow-origin"), Some("*"));
    }

    #[test]
    fn async_route_specific_middleware_runs_around_handler() {
        let route = AsyncRoute::new(Method::Get, "/orders/{id}", |request| {
            async_response(async move {
                Response::ok(request.header("x-order-id").unwrap_or_default().to_owned())
            })
        })
        .with_request_middleware(|request| {
            let order_id = request.path_param("id").unwrap_or_default().to_owned();
            request.with_header("x-order-id", order_id)
        })
        .with_response_middleware(|_, response| response.with_header("x-order", "route"));

        let mut router = AsyncRouter::new();
        router.add_response_middleware(|_, response| response.with_header("x-order", "router"));
        router.register_route(route);

        let response = block_on(router.handle(Request::new(Method::Get, "/orders/order-1")));
        let order_headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| (name == "x-order").then_some(value.as_str()))
            .collect::<Vec<_>>();

        assert_eq!(router.routes()[0].request_middleware_len(), 1);
        assert_eq!(router.routes()[0].response_middleware_len(), 1);
        assert_eq!(response.body(), b"order-1");
        assert_eq!(order_headers, vec!["route", "router"]);
    }

    #[test]
    fn async_include_router_with_prefix_routes_child_paths() {
        let mut child = AsyncRouter::new();
        child.get("/orders/{id}", |request| {
            async_response(async move {
                Response::ok(format!(
                    "order:{}",
                    request.path_param("id").expect("id is captured")
                ))
            })
        });

        let mut router = AsyncRouter::new();
        router.include_router_with_prefix("api", child);

        let response = block_on(router.handle(Request::new(Method::Get, "/api/orders/order-1")));

        assert_eq!(router.len(), 1);
        assert_eq!(router.routes()[0].path(), "/api/orders/{id}");
        assert_eq!(response.body(), b"order:order-1");
    }

    #[test]
    fn async_include_router_merges_not_found_handler() {
        let mut child = AsyncRouter::new();
        child.set_not_found_handler(|_| async_response(async { Response::new(410) }));

        let mut router = AsyncRouter::new()
            .with_not_found_handler(|_| async_response(async { Response::not_found() }));
        router.include_router(child);

        let response = block_on(router.handle(Request::new(Method::Get, "/missing")));

        assert_eq!(response.status_code(), 410);
    }
}
