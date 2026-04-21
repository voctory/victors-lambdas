//! Router facade.

use crate::{Method, Route};

/// Stores route metadata for event handlers.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    /// Creates an empty router.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a route.
    pub fn add_route(&mut self, method: Method, path: impl Into<String>) {
        self.routes.push(Route::new(method, path));
    }

    /// Returns registered routes.
    #[must_use]
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }
}
