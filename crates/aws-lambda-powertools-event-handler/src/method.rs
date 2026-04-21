//! HTTP methods for event routing.

use std::{error::Error, fmt, str::FromStr};

/// HTTP method used for route matching.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Method {
    /// GET request.
    Get,
    /// HEAD request.
    Head,
    /// POST request.
    Post,
    /// PUT request.
    Put,
    /// PATCH request.
    Patch,
    /// DELETE request.
    Delete,
    /// OPTIONS request.
    Options,
    /// Match any request method when used on a route.
    Any,
}

impl Method {
    /// Returns the canonical uppercase HTTP method token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Options => "OPTIONS",
            Self::Any => "ANY",
        }
    }

    /// Returns true when this route method accepts the request method.
    ///
    /// `Method::Any` is a wildcard for registered routes.
    #[must_use]
    pub const fn accepts(self, request_method: Self) -> bool {
        matches!(self, Self::Any)
            || matches!(
                (self, request_method),
                (Self::Get, Self::Get)
                    | (Self::Head, Self::Head)
                    | (Self::Post, Self::Post)
                    | (Self::Put, Self::Put)
                    | (Self::Patch, Self::Patch)
                    | (Self::Delete, Self::Delete)
                    | (Self::Options, Self::Options)
                    | (Self::Any, Self::Any)
            )
    }

    pub(crate) const fn match_score(self, request_method: Self) -> Option<u8> {
        if !self.accepts(request_method) {
            return None;
        }

        if matches!(self, Self::Any) && !matches!(request_method, Self::Any) {
            Some(1)
        } else {
            Some(2)
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for Method {
    type Err = ParseMethodError;

    fn from_str(method: &str) -> Result<Self, Self::Err> {
        if method.eq_ignore_ascii_case("GET") {
            Ok(Self::Get)
        } else if method.eq_ignore_ascii_case("HEAD") {
            Ok(Self::Head)
        } else if method.eq_ignore_ascii_case("POST") {
            Ok(Self::Post)
        } else if method.eq_ignore_ascii_case("PUT") {
            Ok(Self::Put)
        } else if method.eq_ignore_ascii_case("PATCH") {
            Ok(Self::Patch)
        } else if method.eq_ignore_ascii_case("DELETE") {
            Ok(Self::Delete)
        } else if method.eq_ignore_ascii_case("OPTIONS") {
            Ok(Self::Options)
        } else if method.eq_ignore_ascii_case("ANY") {
            Ok(Self::Any)
        } else {
            Err(ParseMethodError {
                method: method.to_owned(),
            })
        }
    }
}

/// Error returned when an HTTP method token is not recognized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseMethodError {
    method: String,
}

impl ParseMethodError {
    /// Returns the method token that could not be parsed.
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }
}

impl fmt::Display for ParseMethodError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unsupported HTTP method: {}", self.method)
    }
}

impl Error for ParseMethodError {}
