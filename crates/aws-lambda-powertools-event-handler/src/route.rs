//! Route definitions.

use std::{error::Error, fmt, future::Future, pin::Pin};

#[cfg(feature = "validation")]
use crate::ValidationConfig;
use crate::{Method, Request, RequestMiddleware, Response, ResponseMiddleware};

/// Function signature used by HTTP routes.
pub type Handler = dyn Fn(&Request) -> Response + Send + Sync + 'static;

/// Error type returned by fallible HTTP route handlers.
pub type RouteError = dyn Error + Send + Sync + 'static;

/// Result type returned by fallible HTTP route handlers.
pub type RouteResult<T = Response> = Result<T, Box<RouteError>>;

/// Function signature used by fallible HTTP routes.
pub type FallibleHandler = dyn Fn(&Request) -> RouteResult + Send + Sync + 'static;

/// Boxed future returned by asynchronous HTTP route handlers.
pub type ResponseFuture<'a> = Pin<Box<dyn Future<Output = Response> + Send + 'a>>;

/// Boxed future returned by asynchronous fallible HTTP route handlers.
pub type FallibleResponseFuture<'a> = Pin<Box<dyn Future<Output = RouteResult> + Send + 'a>>;

/// Function signature used by asynchronous HTTP routes.
pub type AsyncHandler = dyn for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static;

/// Function signature used by asynchronous fallible HTTP routes.
pub type AsyncFallibleHandler =
    dyn for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static;

/// Captured dynamic path parameters for a matched route.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PathParams {
    pairs: Vec<(String, String)>,
}

impl PathParams {
    /// Creates an empty path parameter collection.
    #[must_use]
    pub const fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    /// Creates path parameters from name-value pairs.
    #[must_use]
    pub fn from_pairs<N, V>(pairs: impl IntoIterator<Item = (N, V)>) -> Self
    where
        N: Into<String>,
        V: Into<String>,
    {
        Self {
            pairs: pairs
                .into_iter()
                .map(|(name, value)| (name.into(), value.into()))
                .collect(),
        }
    }

    /// Returns the parameter value for a name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find_map(|(parameter_name, value)| (parameter_name == name).then_some(value.as_str()))
    }

    /// Returns parameter pairs in route order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.pairs
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// Returns the number of captured parameters.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Returns true when no parameters were captured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub(crate) fn push(&mut self, name: &str, value: &str) {
        self.pairs.push((name.to_owned(), value.to_owned()));
    }
}

/// Registered HTTP route.
///
/// Dynamic path parameters use whole path segments wrapped in braces, such as
/// `/orders/{id}`. Partial segment captures are not interpreted.
pub struct Route {
    method: Method,
    path: String,
    pattern: PathPattern,
    handler: RouteHandler,
    request_middleware: Vec<Box<RequestMiddleware>>,
    response_middleware: Vec<Box<ResponseMiddleware>>,
    #[cfg(feature = "validation")]
    validation: Option<ValidationConfig>,
}

impl Route {
    /// Creates a route with a method, path pattern, and handler.
    #[must_use]
    pub fn new(
        method: Method,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) -> Self {
        let path = path.into();

        Self {
            method,
            pattern: PathPattern::parse(&path),
            path,
            handler: RouteHandler::Infallible(Box::new(handler)),
            request_middleware: Vec::new(),
            response_middleware: Vec::new(),
            #[cfg(feature = "validation")]
            validation: None,
        }
    }

    /// Creates a route with a method, path pattern, and fallible handler.
    ///
    /// Errors returned by fallible handlers are handled by the owning
    /// [`Router`](crate::Router) when one is used for dispatch. Calling
    /// [`Route::handle`] directly maps errors to `500 Internal Server Error`;
    /// use [`Route::try_handle`] to observe the original error.
    #[must_use]
    pub fn new_fallible<E>(
        method: Method,
        path: impl Into<String>,
        handler: impl Fn(&Request) -> Result<Response, E> + Send + Sync + 'static,
    ) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        let path = path.into();

        Self {
            method,
            pattern: PathPattern::parse(&path),
            path,
            handler: RouteHandler::Fallible(Box::new(move |request| {
                handler(request).map_err(|error| Box::new(error) as Box<RouteError>)
            })),
            request_middleware: Vec::new(),
            response_middleware: Vec::new(),
            #[cfg(feature = "validation")]
            validation: None,
        }
    }

    /// Returns the route method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns the route path pattern.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns captured parameters when this route matches the method and path.
    #[must_use]
    pub fn matches(&self, method: Method, path: &str) -> Option<PathParams> {
        self.match_request(method, path)
            .map(|route_match| route_match.path_params)
    }

    /// Runs this route's handler.
    #[must_use]
    pub fn handle(&self, request: &Request) -> Response {
        self.try_handle(request)
            .unwrap_or_else(|_| Response::internal_server_error())
    }

    /// Runs this route's handler, preserving fallible route errors.
    ///
    /// Infallible routes return `Ok(Response)`.
    ///
    /// # Errors
    ///
    /// Returns the error from a fallible route handler.
    pub fn try_handle(&self, request: &Request) -> RouteResult {
        self.handler.try_handle(request)
    }

    /// Returns the number of route-specific request middleware functions.
    #[must_use]
    pub fn request_middleware_len(&self) -> usize {
        self.request_middleware.len()
    }

    /// Returns the number of route-specific response middleware functions.
    #[must_use]
    pub fn response_middleware_len(&self) -> usize {
        self.response_middleware.len()
    }

    /// Returns a copy with route-specific request middleware appended.
    ///
    /// Route-specific request middleware runs after route matching and path
    /// parameter capture, but before request validation and the route handler.
    #[must_use]
    pub fn with_request_middleware(
        mut self,
        middleware: impl Fn(Request) -> Request + Send + Sync + 'static,
    ) -> Self {
        self.add_request_middleware(middleware);
        self
    }

    /// Adds route-specific request middleware.
    pub fn add_request_middleware(
        &mut self,
        middleware: impl Fn(Request) -> Request + Send + Sync + 'static,
    ) -> &mut Self {
        self.request_middleware.push(Box::new(middleware));
        self
    }

    /// Returns a copy with route-specific response middleware appended.
    ///
    /// Route-specific response middleware runs after the route handler and
    /// before router-level response middleware.
    #[must_use]
    pub fn with_response_middleware(
        mut self,
        middleware: impl Fn(&Request, Response) -> Response + Send + Sync + 'static,
    ) -> Self {
        self.add_response_middleware(middleware);
        self
    }

    /// Adds route-specific response middleware.
    pub fn add_response_middleware(
        &mut self,
        middleware: impl Fn(&Request, Response) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.response_middleware.push(Box::new(middleware));
        self
    }

    /// Enables validation for this route.
    #[cfg(feature = "validation")]
    #[must_use]
    pub fn with_validation(mut self, validation: ValidationConfig) -> Self {
        self.validation = Some(validation);
        self
    }

    /// Returns this route's validation configuration when enabled.
    #[cfg(feature = "validation")]
    #[must_use]
    pub const fn validation(&self) -> Option<&ValidationConfig> {
        self.validation.as_ref()
    }

    /// Returns a copy with a route-specific request validator appended.
    #[cfg(feature = "validation")]
    #[must_use]
    pub fn with_request_validator(
        mut self,
        validator: impl Fn(&Request) -> aws_lambda_powertools_validation::ValidationResult
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.add_request_validator(validator);
        self
    }

    /// Adds a route-specific request validator, enabling validation when needed.
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

    /// Returns a copy with a route-specific response validator appended.
    #[cfg(feature = "validation")]
    #[must_use]
    pub fn with_response_validator(
        mut self,
        validator: impl Fn(&Request, &Response) -> aws_lambda_powertools_validation::ValidationResult
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.add_response_validator(validator);
        self
    }

    /// Adds a route-specific response validator, enabling validation when needed.
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

    pub(crate) fn match_request(&self, method: Method, path: &str) -> Option<RouteMatchData> {
        let method_score = self.method.match_score(method)?;
        let path_params = self.pattern.matches(path)?;

        Some(RouteMatchData {
            path_params,
            method_score,
        })
    }

    pub(crate) fn path_precedence(&self) -> &[u8] {
        self.pattern.precedence()
    }

    pub(crate) fn apply_request_middleware(&self, mut request: Request) -> Request {
        for middleware in &self.request_middleware {
            request = middleware(request);
        }
        request
    }

    pub(crate) fn apply_response_middleware(
        &self,
        request: &Request,
        mut response: Response,
    ) -> Response {
        for middleware in &self.response_middleware {
            response = middleware(request, response);
        }
        response
    }

    #[cfg(feature = "validation")]
    pub(crate) fn validate_request(
        &self,
        request: &Request,
    ) -> Result<(), aws_lambda_powertools_validation::ValidationError> {
        if let Some(validation) = &self.validation {
            validation.validate_request(request)?;
        }

        Ok(())
    }

    #[cfg(feature = "validation")]
    pub(crate) fn validate_response(
        &self,
        request: &Request,
        response: &Response,
    ) -> Result<(), aws_lambda_powertools_validation::ValidationError> {
        if let Some(validation) = &self.validation {
            validation.validate_response(request, response)?;
        }

        Ok(())
    }

    pub(crate) fn with_path_prefix(mut self, prefix: &str) -> Self {
        let path = join_path_prefix(prefix, &self.path);
        self.pattern = PathPattern::parse(&path);
        self.path = path;
        self
    }
}

impl fmt::Debug for Route {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("Route");
        debug
            .field("method", &self.method)
            .field("path", &self.path)
            .field("request_middleware_len", &self.request_middleware.len())
            .field("response_middleware_len", &self.response_middleware.len());
        #[cfg(feature = "validation")]
        debug.field("validation_enabled", &self.validation.is_some());
        debug.finish_non_exhaustive()
    }
}

/// Registered asynchronous HTTP route.
///
/// Dynamic path parameters use whole path segments wrapped in braces, such as
/// `/orders/{id}`. Partial segment captures are not interpreted.
pub struct AsyncRoute {
    method: Method,
    path: String,
    pattern: PathPattern,
    handler: AsyncRouteHandler,
    request_middleware: Vec<Box<RequestMiddleware>>,
    response_middleware: Vec<Box<ResponseMiddleware>>,
    #[cfg(feature = "validation")]
    validation: Option<ValidationConfig>,
}

impl AsyncRoute {
    /// Creates an asynchronous route with a method, path pattern, and handler.
    #[must_use]
    pub fn new(
        method: Method,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static,
    ) -> Self {
        let path = path.into();

        Self {
            method,
            pattern: PathPattern::parse(&path),
            path,
            handler: AsyncRouteHandler::Infallible(Box::new(handler)),
            request_middleware: Vec::new(),
            response_middleware: Vec::new(),
            #[cfg(feature = "validation")]
            validation: None,
        }
    }

    /// Creates an asynchronous route with a method, path pattern, and fallible handler.
    ///
    /// Errors returned by fallible handlers are handled by the owning
    /// [`AsyncRouter`](crate::AsyncRouter) when one is used for dispatch.
    /// Calling [`AsyncRoute::handle`] directly maps errors to
    /// `500 Internal Server Error`; use [`AsyncRoute::try_handle`] to observe
    /// the original error.
    #[must_use]
    pub fn new_fallible(
        method: Method,
        path: impl Into<String>,
        handler: impl for<'a> Fn(&'a Request) -> FallibleResponseFuture<'a> + Send + Sync + 'static,
    ) -> Self {
        let path = path.into();

        Self {
            method,
            pattern: PathPattern::parse(&path),
            path,
            handler: AsyncRouteHandler::Fallible(Box::new(handler)),
            request_middleware: Vec::new(),
            response_middleware: Vec::new(),
            #[cfg(feature = "validation")]
            validation: None,
        }
    }

    /// Returns the route method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns the route path pattern.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns captured parameters when this route matches the method and path.
    #[must_use]
    pub fn matches(&self, method: Method, path: &str) -> Option<PathParams> {
        self.match_request(method, path)
            .map(|route_match| route_match.path_params)
    }

    /// Runs this route's asynchronous handler.
    pub fn handle<'a>(&'a self, request: &'a Request) -> ResponseFuture<'a> {
        Box::pin(async move {
            self.try_handle(request)
                .await
                .unwrap_or_else(|_| Response::internal_server_error())
        })
    }

    /// Runs this asynchronous route's handler, preserving fallible route errors.
    ///
    /// Infallible routes return `Ok(Response)`.
    ///
    /// # Errors
    ///
    /// Returns the error from a fallible route handler.
    pub fn try_handle<'a>(&'a self, request: &'a Request) -> FallibleResponseFuture<'a> {
        self.handler.try_handle(request)
    }

    /// Returns the number of route-specific request middleware functions.
    #[must_use]
    pub fn request_middleware_len(&self) -> usize {
        self.request_middleware.len()
    }

    /// Returns the number of route-specific response middleware functions.
    #[must_use]
    pub fn response_middleware_len(&self) -> usize {
        self.response_middleware.len()
    }

    /// Returns a copy with route-specific request middleware appended.
    ///
    /// Route-specific request middleware runs after route matching and path
    /// parameter capture, but before request validation and the route handler.
    #[must_use]
    pub fn with_request_middleware(
        mut self,
        middleware: impl Fn(Request) -> Request + Send + Sync + 'static,
    ) -> Self {
        self.add_request_middleware(middleware);
        self
    }

    /// Adds route-specific request middleware.
    pub fn add_request_middleware(
        &mut self,
        middleware: impl Fn(Request) -> Request + Send + Sync + 'static,
    ) -> &mut Self {
        self.request_middleware.push(Box::new(middleware));
        self
    }

    /// Returns a copy with route-specific response middleware appended.
    ///
    /// Route-specific response middleware runs after the route handler and
    /// before router-level response middleware.
    #[must_use]
    pub fn with_response_middleware(
        mut self,
        middleware: impl Fn(&Request, Response) -> Response + Send + Sync + 'static,
    ) -> Self {
        self.add_response_middleware(middleware);
        self
    }

    /// Adds route-specific response middleware.
    pub fn add_response_middleware(
        &mut self,
        middleware: impl Fn(&Request, Response) -> Response + Send + Sync + 'static,
    ) -> &mut Self {
        self.response_middleware.push(Box::new(middleware));
        self
    }

    /// Enables validation for this asynchronous route.
    #[cfg(feature = "validation")]
    #[must_use]
    pub fn with_validation(mut self, validation: ValidationConfig) -> Self {
        self.validation = Some(validation);
        self
    }

    /// Returns this asynchronous route's validation configuration when enabled.
    #[cfg(feature = "validation")]
    #[must_use]
    pub const fn validation(&self) -> Option<&ValidationConfig> {
        self.validation.as_ref()
    }

    /// Returns a copy with a route-specific request validator appended.
    #[cfg(feature = "validation")]
    #[must_use]
    pub fn with_request_validator(
        mut self,
        validator: impl Fn(&Request) -> aws_lambda_powertools_validation::ValidationResult
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.add_request_validator(validator);
        self
    }

    /// Adds a route-specific request validator, enabling validation when needed.
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

    /// Returns a copy with a route-specific response validator appended.
    #[cfg(feature = "validation")]
    #[must_use]
    pub fn with_response_validator(
        mut self,
        validator: impl Fn(&Request, &Response) -> aws_lambda_powertools_validation::ValidationResult
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.add_response_validator(validator);
        self
    }

    /// Adds a route-specific response validator, enabling validation when needed.
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

    pub(crate) fn match_request(&self, method: Method, path: &str) -> Option<RouteMatchData> {
        let method_score = self.method.match_score(method)?;
        let path_params = self.pattern.matches(path)?;

        Some(RouteMatchData {
            path_params,
            method_score,
        })
    }

    pub(crate) fn path_precedence(&self) -> &[u8] {
        self.pattern.precedence()
    }

    pub(crate) fn apply_request_middleware(&self, mut request: Request) -> Request {
        for middleware in &self.request_middleware {
            request = middleware(request);
        }
        request
    }

    pub(crate) fn apply_response_middleware(
        &self,
        request: &Request,
        mut response: Response,
    ) -> Response {
        for middleware in &self.response_middleware {
            response = middleware(request, response);
        }
        response
    }

    #[cfg(feature = "validation")]
    pub(crate) fn validate_request(
        &self,
        request: &Request,
    ) -> Result<(), aws_lambda_powertools_validation::ValidationError> {
        if let Some(validation) = &self.validation {
            validation.validate_request(request)?;
        }

        Ok(())
    }

    #[cfg(feature = "validation")]
    pub(crate) fn validate_response(
        &self,
        request: &Request,
        response: &Response,
    ) -> Result<(), aws_lambda_powertools_validation::ValidationError> {
        if let Some(validation) = &self.validation {
            validation.validate_response(request, response)?;
        }

        Ok(())
    }

    pub(crate) fn with_path_prefix(mut self, prefix: &str) -> Self {
        let path = join_path_prefix(prefix, &self.path);
        self.pattern = PathPattern::parse(&path);
        self.path = path;
        self
    }
}

impl fmt::Debug for AsyncRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("AsyncRoute");
        debug
            .field("method", &self.method)
            .field("path", &self.path)
            .field("request_middleware_len", &self.request_middleware.len())
            .field("response_middleware_len", &self.response_middleware.len());
        #[cfg(feature = "validation")]
        debug.field("validation_enabled", &self.validation.is_some());
        debug.finish_non_exhaustive()
    }
}

pub(crate) struct RouteMatchData {
    pub(crate) path_params: PathParams,
    pub(crate) method_score: u8,
}

enum RouteHandler {
    Infallible(Box<Handler>),
    Fallible(Box<FallibleHandler>),
}

impl RouteHandler {
    fn try_handle(&self, request: &Request) -> RouteResult {
        match self {
            Self::Infallible(handler) => Ok(handler(request)),
            Self::Fallible(handler) => handler(request),
        }
    }
}

enum AsyncRouteHandler {
    Infallible(Box<AsyncHandler>),
    Fallible(Box<AsyncFallibleHandler>),
}

impl AsyncRouteHandler {
    fn try_handle<'a>(&'a self, request: &'a Request) -> FallibleResponseFuture<'a> {
        match self {
            Self::Infallible(handler) => {
                let response = handler(request);
                Box::pin(async move { Ok(response.await) })
            }
            Self::Fallible(handler) => handler(request),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PathPattern {
    segments: Vec<PathSegment>,
    precedence: Vec<u8>,
}

impl PathPattern {
    fn parse(path: &str) -> Self {
        let segments: Vec<PathSegment> = split_path(path)
            .into_iter()
            .map(PathSegment::parse)
            .collect();
        let precedence = segments
            .iter()
            .map(|segment| u8::from(matches!(segment, PathSegment::Static(_))))
            .collect();

        Self {
            segments,
            precedence,
        }
    }

    fn matches(&self, path: &str) -> Option<PathParams> {
        let request_segments = split_path(path);

        if self.segments.len() != request_segments.len() {
            return None;
        }

        let mut path_params = PathParams::new();

        for (route_segment, request_segment) in self.segments.iter().zip(request_segments) {
            match route_segment {
                PathSegment::Static(expected) if expected == request_segment => {}
                PathSegment::Static(_) => return None,
                PathSegment::Parameter(name) => path_params.push(name, request_segment),
            }
        }

        Some(path_params)
    }

    fn precedence(&self) -> &[u8] {
        &self.precedence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PathSegment {
    Static(String),
    Parameter(String),
}

impl PathSegment {
    fn parse(segment: &str) -> Self {
        if let Some(name) = segment
            .strip_prefix('{')
            .and_then(|without_prefix| without_prefix.strip_suffix('}'))
            .filter(|name| !name.is_empty())
        {
            Self::Parameter(name.to_owned())
        } else {
            Self::Static(segment.to_owned())
        }
    }
}

fn split_path(path: &str) -> Vec<&str> {
    if path == "/" {
        Vec::new()
    } else {
        path.strip_prefix('/').unwrap_or(path).split('/').collect()
    }
}

fn join_path_prefix(prefix: &str, path: &str) -> String {
    let prefix = prefix.trim_matches('/');
    if prefix.is_empty() {
        return path.to_owned();
    }

    let path = path.trim_matches('/');
    if path.is_empty() {
        format!("/{prefix}")
    } else {
        format!("/{prefix}/{path}")
    }
}
