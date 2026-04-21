//! Route definitions.

use crate::Method;

/// Route metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    method: Method,
    path: String,
}

impl Route {
    /// Creates route metadata.
    #[must_use]
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
        }
    }

    /// Returns the route method.
    #[must_use]
    pub fn method(&self) -> Method {
        self.method
    }

    /// Returns the route path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}
