//! Router facade.

use std::{cmp::Ordering, fmt};

#[cfg(feature = "validation")]
use crate::validation::{
    ValidationConfig, request_validation_response, response_validation_response,
};
use crate::{
    AsyncHandler, AsyncRoute, CorsConfig, Handler, Method, Request, Response, ResponseFuture, Route,
};

/// Function signature used by request middleware.
pub type RequestMiddleware = dyn Fn(Request) -> Request + Send + Sync + 'static;

/// Function signature used by response middleware.
pub type ResponseMiddleware = dyn Fn(&Request, Response) -> Response + Send + Sync + 'static;

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
    not_found_handler: Option<Box<Handler>>,
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
    not_found_handler: Option<Box<AsyncHandler>>,
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
            not_found_handler: None,
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

    /// Adds a `GET` route handler.
    pub fn get(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Get, path, handler)
    }

    /// Adds a `HEAD` route handler.
    pub fn head(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Head, path, handler)
    }

    /// Adds a `POST` route handler.
    pub fn post(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Post, path, handler)
    }

    /// Adds a `PUT` route handler.
    pub fn put(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Put, path, handler)
    }

    /// Adds a `PATCH` route handler.
    pub fn patch(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Patch, path, handler)
    }

    /// Adds a `DELETE` route handler.
    pub fn delete(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Delete, path, handler)
    }

    /// Adds an `OPTIONS` route handler.
    pub fn options(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Options, path, handler)
    }

    /// Adds a route handler that accepts any request method.
    pub fn any(
        &mut self,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Any, path, handler)
    }

    /// Includes routes, middleware, validation hooks, and not-found handling from another router.
    ///
    /// Included routes behave as if they were registered after this router's
    /// existing routes. The included router's CORS configuration is not merged.
    /// If the included router has a not-found handler, it replaces this
    /// router's not-found handler.
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
            not_found_handler,
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
        if let Some(not_found_handler) = not_found_handler {
            self.not_found_handler = Some(not_found_handler);
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
        request = self.apply_request_middleware(request);

        if let Some(cors) = &self.cors {
            if cors.is_preflight_request(&request) {
                return cors.preflight_response();
            }
        }

        let Some(route_match) = self.find(&request) else {
            let response = self.not_found_response(&request);
            let response = self.apply_response_middleware(&request, response);
            return self.apply_cors(response);
        };
        let route = route_match.route;

        request.set_path_params(&route_match.path_params);
        #[cfg(feature = "validation")]
        if let Some(response) = self.validate_request(&request) {
            return self.apply_cors(response);
        }

        let response = self.apply_response_middleware(&request, route.handle(&request));
        #[cfg(feature = "validation")]
        if let Some(validation_response) = self.validate_response(&request, &response) {
            return self.apply_cors(validation_response);
        }

        self.apply_cors(response)
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

    fn apply_cors(&self, response: Response) -> Response {
        if let Some(cors) = &self.cors {
            cors.apply(response)
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
            not_found_handler: None,
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

    /// Adds an asynchronous `GET` route handler.
    pub fn get(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Get, path, handler)
    }

    /// Adds an asynchronous `HEAD` route handler.
    pub fn head(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Head, path, handler)
    }

    /// Adds an asynchronous `POST` route handler.
    pub fn post(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Post, path, handler)
    }

    /// Adds an asynchronous `PUT` route handler.
    pub fn put(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Put, path, handler)
    }

    /// Adds an asynchronous `PATCH` route handler.
    pub fn patch(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Patch, path, handler)
    }

    /// Adds an asynchronous `DELETE` route handler.
    pub fn delete(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Delete, path, handler)
    }

    /// Adds an asynchronous `OPTIONS` route handler.
    pub fn options(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Options, path, handler)
    }

    /// Adds an asynchronous route handler that accepts any request method.
    pub fn any(
        &mut self,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> &mut Self {
        self.add_route(Method::Any, path, handler)
    }

    /// Includes routes, middleware, validation hooks, and not-found handling from another asynchronous router.
    ///
    /// Included routes behave as if they were registered after this router's
    /// existing routes. The included router's CORS configuration is not merged.
    /// If the included router has a not-found handler, it replaces this
    /// router's not-found handler.
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
            not_found_handler,
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
        if let Some(not_found_handler) = not_found_handler {
            self.not_found_handler = Some(not_found_handler);
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
        request = self.apply_request_middleware(request);

        if let Some(cors) = &self.cors {
            if cors.is_preflight_request(&request) {
                return cors.preflight_response();
            }
        }

        let Some(route_match) = self.find(&request) else {
            let response = self.not_found_response(&request).await;
            let response = self.apply_response_middleware(&request, response);
            return self.apply_cors(response);
        };
        let route = route_match.route;

        request.set_path_params(&route_match.path_params);
        #[cfg(feature = "validation")]
        if let Some(response) = self.validate_request(&request) {
            return self.apply_cors(response);
        }

        let response = self.apply_response_middleware(&request, route.handle(&request).await);
        #[cfg(feature = "validation")]
        if let Some(validation_response) = self.validate_response(&request, &response) {
            return self.apply_cors(validation_response);
        }

        self.apply_cors(response)
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

    fn apply_cors(&self, response: Response) -> Response {
        if let Some(cors) = &self.cors {
            cors.apply(response)
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
            .field("has_not_found_handler", &self.not_found_handler.is_some());
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
            .field("has_not_found_handler", &self.not_found_handler.is_some());
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

#[cfg(test)]
mod tests {
    use std::future::Future;

    #[cfg(feature = "validation")]
    use aws_lambda_powertools_validation::Validator;
    use futures_executor::block_on;

    #[cfg(feature = "validation")]
    use crate::ValidationConfig;
    use crate::{AsyncRouter, CorsConfig, Method, Request, Response, ResponseFuture, Router};

    fn async_response<'a>(
        future: impl Future<Output = Response> + Send + 'a,
    ) -> ResponseFuture<'a> {
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
    fn cors_headers_are_added_to_routed_and_not_found_responses() {
        let mut router = Router::new().with_cors(CorsConfig::new("https://example.com"));
        router.get("/orders", |_| Response::ok("orders"));

        let orders_response = router.handle(Request::new(Method::Get, "/orders"));
        let not_found = router.handle(Request::new(Method::Get, "/missing"));

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

        let response = router.handle(Request::new(Method::Get, "/orders"));

        assert_eq!(router.response_middleware_len(), 1);
        assert_eq!(response.header("x-path"), Some("/orders"));
        assert_eq!(response.header("access-control-allow-origin"), Some("*"));
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
    fn async_router_runs_middleware_and_cors() {
        let mut router = AsyncRouter::new().with_cors(CorsConfig::default());
        router.add_request_middleware(|request| {
            if request.path() == "/legacy-orders" {
                Request::new(request.method(), "/orders")
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

        let response = block_on(router.handle(Request::new(Method::Get, "/legacy-orders")));

        assert_eq!(router.request_middleware_len(), 1);
        assert_eq!(router.response_middleware_len(), 1);
        assert_eq!(response.body(), b"/orders");
        assert_eq!(response.header("x-path"), Some("/orders"));
        assert_eq!(response.header("access-control-allow-origin"), Some("*"));
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
