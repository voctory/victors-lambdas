//! HTTP response compression middleware.

use std::io::Write;

use flate2::{
    Compression,
    write::{GzEncoder, ZlibEncoder},
};

use crate::{Method, Request, Response};

/// Default minimum response size for compression.
pub const DEFAULT_COMPRESSION_THRESHOLD: usize = 1024;

/// Supported HTTP response compression encodings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CompressionEncoding {
    /// Gzip content encoding.
    #[default]
    Gzip,
    /// Deflate content encoding.
    Deflate,
}

impl CompressionEncoding {
    /// Returns the HTTP `Content-Encoding` header value.
    #[must_use]
    pub const fn header_value(self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Deflate => "deflate",
        }
    }
}

/// HTTP response compression configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompressionConfig {
    threshold: usize,
    encoding: CompressionEncoding,
}

impl CompressionConfig {
    /// Creates compression configuration with the default threshold and gzip.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            threshold: DEFAULT_COMPRESSION_THRESHOLD,
            encoding: CompressionEncoding::Gzip,
        }
    }

    /// Returns the minimum response size required for compression.
    #[must_use]
    pub const fn threshold(&self) -> usize {
        self.threshold
    }

    /// Returns the preferred compression encoding.
    #[must_use]
    pub const fn encoding(&self) -> CompressionEncoding {
        self.encoding
    }

    /// Returns a copy with a different minimum response size.
    #[must_use]
    pub const fn with_threshold(mut self, threshold: usize) -> Self {
        self.threshold = threshold;
        self
    }

    /// Returns a copy with a different preferred compression encoding.
    #[must_use]
    pub const fn with_encoding(mut self, encoding: CompressionEncoding) -> Self {
        self.encoding = encoding;
        self
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds response middleware that compresses eligible response bodies.
pub fn compression_middleware(
    config: CompressionConfig,
) -> impl Fn(&Request, Response) -> Response + Clone + Send + Sync + 'static {
    move |request, response| compress_response(request, response, config)
}

/// Compresses a response body when request and response headers allow it.
#[must_use]
pub fn compress_response(
    request: &Request,
    response: Response,
    config: CompressionConfig,
) -> Response {
    if !should_compress(request, &response, config) {
        return response;
    }

    let Some(body) = encode_body(response.body(), config.encoding()).ok() else {
        return response;
    };

    response
        .without_header("content-length")
        .with_replaced_header("content-encoding", config.encoding().header_value())
        .with_body(body)
}

fn should_compress(request: &Request, response: &Response, config: CompressionConfig) -> bool {
    accepts_encoding(request, config.encoding())
        && request.method() != Method::Head
        && response.body().len() > config.threshold()
        && response.header("content-encoding").is_none()
        && response.header("transfer-encoding").is_none()
        && !has_no_transform(response)
}

fn accepts_encoding(request: &Request, encoding: CompressionEncoding) -> bool {
    let Some(value) = request.header("accept-encoding") else {
        return true;
    };

    let mut accepts = false;
    for item in value.split(',') {
        let token = item.trim().split(';').next().unwrap_or_default().trim();

        if token.eq_ignore_ascii_case("identity") {
            return false;
        }

        if token == "*" || token.eq_ignore_ascii_case(encoding.header_value()) {
            accepts = true;
        }
    }

    accepts
}

fn has_no_transform(response: &Response) -> bool {
    response.header("cache-control").is_some_and(|value| {
        value
            .split(',')
            .any(|item| item.trim().eq_ignore_ascii_case("no-transform"))
    })
}

fn encode_body(body: &[u8], encoding: CompressionEncoding) -> std::io::Result<Vec<u8>> {
    match encoding {
        CompressionEncoding::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(body)?;
            encoder.finish()
        }
        CompressionEncoding::Deflate => {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(body)?;
            encoder.finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use flate2::read::{GzDecoder, ZlibDecoder};

    use super::{
        CompressionConfig, CompressionEncoding, compress_response, compression_middleware,
    };
    use crate::{Method, Request, Response, Router};

    #[test]
    fn compresses_gzip_response_when_accepted() {
        let request = Request::new(Method::Get, "/orders").with_header("accept-encoding", "gzip");
        let response = Response::ok("orders".repeat(300)).with_header("content-length", "1800");

        let response = compress_response(&request, response, CompressionConfig::default());

        assert_eq!(response.header("content-encoding"), Some("gzip"));
        assert!(response.header("content-length").is_none());
        assert_eq!(decode_gzip(response.body()), "orders".repeat(300));
    }

    #[test]
    fn compresses_deflate_response_when_configured() {
        let request =
            Request::new(Method::Get, "/orders").with_header("accept-encoding", "deflate");
        let response = Response::ok("orders".repeat(300));
        let config = CompressionConfig::default().with_encoding(CompressionEncoding::Deflate);

        let response = compress_response(&request, response, config);

        assert_eq!(response.header("content-encoding"), Some("deflate"));
        assert_eq!(decode_deflate(response.body()), "orders".repeat(300));
    }

    #[test]
    fn skips_compression_when_response_is_below_threshold() {
        let request = Request::new(Method::Get, "/orders").with_header("accept-encoding", "gzip");
        let response = Response::ok("small");
        let config = CompressionConfig::default().with_threshold(100);

        let response = compress_response(&request, response, config);

        assert_eq!(response.header("content-encoding"), None);
        assert_eq!(response.body(), b"small");
    }

    #[test]
    fn skips_compression_when_headers_disallow_it() {
        let body = "orders".repeat(300);
        let request =
            Request::new(Method::Get, "/orders").with_header("accept-encoding", "identity");
        let response = Response::ok(body.clone());

        let response = compress_response(&request, response, CompressionConfig::default());

        assert_eq!(response.header("content-encoding"), None);
        assert_eq!(response.body(), body.as_bytes());
    }

    #[test]
    fn skips_compression_for_head_requests_and_no_transform_responses() {
        let response = compress_response(
            &Request::new(Method::Head, "/orders").with_header("accept-encoding", "gzip"),
            Response::ok("orders".repeat(300)),
            CompressionConfig::default(),
        );
        assert_eq!(response.header("content-encoding"), None);

        let response = compress_response(
            &Request::new(Method::Get, "/orders").with_header("accept-encoding", "gzip"),
            Response::ok("orders".repeat(300)).with_header("cache-control", "public,no-transform"),
            CompressionConfig::default(),
        );
        assert_eq!(response.header("content-encoding"), None);
    }

    #[test]
    fn compression_middleware_runs_with_router_response_middleware() {
        let mut router = Router::new();
        router.add_response_middleware(compression_middleware(
            CompressionConfig::default().with_threshold(10),
        ));
        router.get("/orders", |_| Response::ok("orders".repeat(30)));

        let response = router
            .handle(Request::new(Method::Get, "/orders").with_header("accept-encoding", "gzip"));

        assert_eq!(response.header("content-encoding"), Some("gzip"));
        assert_eq!(decode_gzip(response.body()), "orders".repeat(30));
    }

    fn decode_gzip(body: &[u8]) -> String {
        let mut decoder = GzDecoder::new(body);
        let mut output = String::new();
        decoder
            .read_to_string(&mut output)
            .expect("gzip body should decode");
        output
    }

    fn decode_deflate(body: &[u8]) -> String {
        let mut decoder = ZlibDecoder::new(body);
        let mut output = String::new();
        decoder
            .read_to_string(&mut output)
            .expect("deflate body should decode");
        output
    }
}
