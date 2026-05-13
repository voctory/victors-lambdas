//! Route definitions.

use std::{fmt, future::Future, pin::Pin};

use crate::{Method, Request, Response};

/// Function signature used by HTTP routes.
pub type Handler = dyn Fn(&Request) -> Response + Send + Sync + 'static;

/// Boxed future returned by asynchronous HTTP route handlers.
pub type ResponseFuture<'a> = Pin<Box<dyn Future<Output = Response> + Send + 'a>>;

/// Function signature used by asynchronous HTTP routes.
pub type AsyncHandler = dyn for<'a> Fn(&'a Request) -> ResponseFuture<'a> + Send + Sync + 'static;

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
    handler: Box<Handler>,
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
            handler: Box::new(handler),
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
        (self.handler)(request)
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
}

impl fmt::Debug for Route {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Route")
            .field("method", &self.method)
            .field("path", &self.path)
            .finish_non_exhaustive()
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
    handler: Box<AsyncHandler>,
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
            handler: Box::new(handler),
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
        (self.handler)(request)
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
}

impl fmt::Debug for AsyncRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AsyncRoute")
            .field("method", &self.method)
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

pub(crate) struct RouteMatchData {
    pub(crate) path_params: PathParams,
    pub(crate) method_score: u8,
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
