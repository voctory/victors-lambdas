//! Router facade.

use std::{cmp::Ordering, fmt};

use crate::{CorsConfig, Method, Request, Response, Route};

/// Stores HTTP route handlers and selects the most specific matching route.
///
/// Route precedence is path-first: static path segments take precedence over
/// dynamic path parameters, then exact method routes take precedence over
/// `Method::Any`. Ties preserve registration order.
#[derive(Default)]
pub struct Router {
    routes: Vec<Route>,
    cors: Option<CorsConfig>,
}

impl Router {
    /// Creates an empty router.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            routes: Vec::new(),
            cors: None,
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
        if let Some(cors) = &self.cors {
            if cors.is_preflight_request(&request) {
                return cors.preflight_response();
            }
        }

        let Some(route_match) = self.find(&request) else {
            return self.apply_cors(Response::not_found());
        };
        let route = route_match.route;

        request.set_path_params(route_match.path_params);
        self.apply_cors(route.handle(&request))
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
}

impl fmt::Debug for Router {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Router")
            .field("routes", &self.routes)
            .field("cors", &self.cors)
            .finish()
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

struct SelectedRoute<'a> {
    route: &'a Route,
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

#[cfg(test)]
mod tests {
    use crate::{CorsConfig, Method, Request, Response, Router};

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
}
