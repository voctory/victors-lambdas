//! Lambda execution environment metadata utility.

use std::{
    error::Error,
    fmt,
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{Mutex, MutexGuard, OnceLock, PoisonError},
    time::Duration,
};

use serde_json::{Map, Value};
use victors_lambdas_core::env;

/// Lambda metadata endpoint API version.
pub const LAMBDA_METADATA_API_VERSION: &str = "2026-01-15";

/// Lambda execution-environment metadata endpoint path.
pub const LAMBDA_METADATA_PATH: &str = "/metadata/execution-environment";

/// Default timeout for metadata endpoint requests.
pub const DEFAULT_LAMBDA_METADATA_TIMEOUT: Duration = Duration::from_secs(1);

static METADATA_CACHE: OnceLock<Mutex<Option<LambdaMetadata>>> = OnceLock::new();

/// Result returned by Lambda metadata operations.
pub type LambdaMetadataResult<T> = Result<T, LambdaMetadataError>;

/// Error category for Lambda metadata operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LambdaMetadataErrorKind {
    /// Required environment configuration is missing.
    MissingEnvironment,
    /// Endpoint or token configuration is invalid.
    InvalidConfiguration,
    /// The metadata endpoint request failed.
    Request,
    /// The endpoint returned a malformed HTTP response.
    InvalidResponse,
    /// The endpoint returned a non-success HTTP status.
    HttpStatus,
    /// The endpoint response body could not be parsed.
    Parse,
}

/// Error returned when Lambda metadata cannot be loaded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LambdaMetadataError {
    kind: LambdaMetadataErrorKind,
    message: String,
    status_code: Option<u16>,
}

impl LambdaMetadataError {
    /// Creates a Lambda metadata error.
    #[must_use]
    pub fn new(kind: LambdaMetadataErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            status_code: None,
        }
    }

    /// Creates a Lambda metadata error for a non-success HTTP status.
    #[must_use]
    pub fn http_status(status_code: u16) -> Self {
        Self {
            kind: LambdaMetadataErrorKind::HttpStatus,
            message: format!("metadata request failed with status {status_code}"),
            status_code: Some(status_code),
        }
    }

    /// Returns the error category.
    #[must_use]
    pub const fn kind(&self) -> LambdaMetadataErrorKind {
        self.kind
    }

    /// Returns the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the HTTP status code when the endpoint returned an error status.
    #[must_use]
    pub const fn status_code(&self) -> Option<u16> {
        self.status_code
    }
}

impl fmt::Display for LambdaMetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for LambdaMetadataError {}

/// Lambda execution environment metadata.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LambdaMetadata {
    availability_zone_id: Option<String>,
    raw: Map<String, Value>,
}

impl LambdaMetadata {
    /// Creates metadata from a raw endpoint object.
    #[must_use]
    pub fn from_raw(raw: Map<String, Value>) -> Self {
        let availability_zone_id = raw
            .get("AvailabilityZoneID")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        Self {
            availability_zone_id,
            raw,
        }
    }

    /// Returns the Availability Zone ID where the function is executing.
    #[must_use]
    pub fn availability_zone_id(&self) -> Option<&str> {
        self.availability_zone_id.as_deref()
    }

    /// Returns the raw metadata object for forward-compatible fields.
    #[must_use]
    pub const fn raw(&self) -> &Map<String, Value> {
        &self.raw
    }

    /// Returns whether the metadata object has no endpoint fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }
}

/// Client for the Lambda execution-environment metadata endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LambdaMetadataClient {
    timeout: Duration,
}

impl LambdaMetadataClient {
    /// Creates a metadata client with the default timeout.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            timeout: DEFAULT_LAMBDA_METADATA_TIMEOUT,
        }
    }

    /// Creates a metadata client with a custom timeout.
    ///
    /// A zero timeout is normalized to the default timeout.
    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            timeout: normalized_timeout(timeout),
        }
    }

    /// Returns the configured request timeout.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Loads metadata using process environment variables.
    ///
    /// Returns empty metadata when not running in Lambda or when `POWERTOOLS_DEV`
    /// is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error when required Lambda metadata environment variables are
    /// missing in Lambda, the endpoint request fails, or the response cannot be
    /// parsed.
    pub fn fetch(&self) -> LambdaMetadataResult<LambdaMetadata> {
        self.fetch_with_env(env::var)
    }

    /// Loads metadata using an injected environment source.
    ///
    /// This is useful for tests and callers that do not want to read process
    /// environment variables directly.
    ///
    /// # Errors
    ///
    /// Returns an error when required Lambda metadata environment variables are
    /// missing in Lambda, the endpoint request fails, or the response cannot be
    /// parsed.
    pub fn fetch_with_env(
        &self,
        source: impl FnMut(&str) -> Option<String>,
    ) -> LambdaMetadataResult<LambdaMetadata> {
        match metadata_request_from_env(source)? {
            MetadataRequest::Skip => Ok(LambdaMetadata::default()),
            MetadataRequest::Fetch { endpoint, token } => {
                self.fetch_from_endpoint(&endpoint, &token)
            }
        }
    }

    /// Loads metadata from a concrete Lambda metadata endpoint authority.
    ///
    /// The endpoint should be the host and port from `AWS_LAMBDA_METADATA_API`,
    /// for example `127.0.0.1:9001`.
    ///
    /// # Errors
    ///
    /// Returns an error when the endpoint or token are invalid, the endpoint
    /// request fails, or the response cannot be parsed.
    pub fn fetch_from_endpoint(
        &self,
        endpoint: &str,
        token: &str,
    ) -> LambdaMetadataResult<LambdaMetadata> {
        let endpoint = normalize_required(env::AWS_LAMBDA_METADATA_API, endpoint)?;
        let token = normalize_required(env::AWS_LAMBDA_METADATA_TOKEN, token)?;
        reject_header_value(env::AWS_LAMBDA_METADATA_API, &endpoint)?;
        reject_header_value(env::AWS_LAMBDA_METADATA_TOKEN, &token)?;

        let response = request_metadata(&endpoint, &token, self.timeout)?;
        let body = parse_http_response(&response)?;
        metadata_from_body(&body)
    }
}

impl Default for LambdaMetadataClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Retrieves Lambda execution environment metadata with process-level caching.
///
/// Returns empty metadata when not running in Lambda or when `POWERTOOLS_DEV` is
/// enabled. Successful endpoint responses are cached for the process lifetime.
///
/// # Errors
///
/// Returns an error when required Lambda metadata environment variables are
/// missing in Lambda, the endpoint request fails, or the response cannot be
/// parsed.
pub fn get_lambda_metadata() -> LambdaMetadataResult<LambdaMetadata> {
    get_lambda_metadata_with_timeout(DEFAULT_LAMBDA_METADATA_TIMEOUT)
}

/// Retrieves Lambda execution environment metadata with a custom timeout.
///
/// Returns empty metadata when not running in Lambda or when `POWERTOOLS_DEV` is
/// enabled. Successful endpoint responses are cached for the process lifetime.
///
/// # Errors
///
/// Returns an error when required Lambda metadata environment variables are
/// missing in Lambda, the endpoint request fails, or the response cannot be
/// parsed.
pub fn get_lambda_metadata_with_timeout(timeout: Duration) -> LambdaMetadataResult<LambdaMetadata> {
    let request = metadata_request_from_env(env::var)?;
    let MetadataRequest::Fetch { endpoint, token } = request else {
        return Ok(LambdaMetadata::default());
    };

    let mut cache = lock_metadata_cache();
    if let Some(metadata) = cache.clone() {
        return Ok(metadata);
    }

    let metadata =
        LambdaMetadataClient::with_timeout(timeout).fetch_from_endpoint(&endpoint, &token)?;
    *cache = Some(metadata.clone());
    Ok(metadata)
}

/// Clears the cached Lambda metadata and returns whether cached metadata existed.
pub fn clear_lambda_metadata_cache() -> bool {
    let mut cache = lock_metadata_cache();
    cache.take().is_some()
}

enum MetadataRequest {
    Skip,
    Fetch { endpoint: String, token: String },
}

fn metadata_request_from_env(
    mut source: impl FnMut(&str) -> Option<String>,
) -> LambdaMetadataResult<MetadataRequest> {
    if source(env::POWERTOOLS_DEV).is_some_and(|value| env::is_truthy(&value)) {
        return Ok(MetadataRequest::Skip);
    }

    let initialization_type = source(env::AWS_LAMBDA_INITIALIZATION_TYPE);
    if initialization_type
        .as_deref()
        .and_then(normalize_env_value)
        .is_none()
    {
        return Ok(MetadataRequest::Skip);
    }

    let endpoint = source(env::AWS_LAMBDA_METADATA_API);
    let endpoint = endpoint
        .as_deref()
        .and_then(normalize_env_value)
        .ok_or_else(|| missing_env_error(env::AWS_LAMBDA_METADATA_API))?;
    let token = source(env::AWS_LAMBDA_METADATA_TOKEN);
    let token = token
        .as_deref()
        .and_then(normalize_env_value)
        .ok_or_else(|| missing_env_error(env::AWS_LAMBDA_METADATA_TOKEN))?;

    Ok(MetadataRequest::Fetch { endpoint, token })
}

fn missing_env_error(name: &str) -> LambdaMetadataError {
    LambdaMetadataError::new(
        LambdaMetadataErrorKind::MissingEnvironment,
        format!("environment variable {name} is not set"),
    )
}

fn normalize_env_value(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn normalize_required(name: &str, value: &str) -> LambdaMetadataResult<String> {
    let value = value.trim();
    if value.is_empty() {
        Err(LambdaMetadataError::new(
            LambdaMetadataErrorKind::InvalidConfiguration,
            format!("{name} cannot be empty"),
        ))
    } else {
        Ok(value.to_owned())
    }
}

fn reject_header_value(name: &str, value: &str) -> LambdaMetadataResult<()> {
    if value.contains(['\r', '\n']) {
        Err(LambdaMetadataError::new(
            LambdaMetadataErrorKind::InvalidConfiguration,
            format!("{name} cannot contain line breaks"),
        ))
    } else {
        Ok(())
    }
}

fn normalized_timeout(timeout: Duration) -> Duration {
    if timeout.is_zero() {
        DEFAULT_LAMBDA_METADATA_TIMEOUT
    } else {
        timeout
    }
}

fn request_metadata(
    endpoint: &str,
    token: &str,
    timeout: Duration,
) -> LambdaMetadataResult<Vec<u8>> {
    let timeout = normalized_timeout(timeout);
    let request = format!(
        "GET /{LAMBDA_METADATA_API_VERSION}{LAMBDA_METADATA_PATH} HTTP/1.1\r\n\
         Host: {endpoint}\r\n\
         Authorization: Bearer {token}\r\n\
         Accept: application/json\r\n\
         Connection: close\r\n\r\n"
    );

    let mut stream = connect(endpoint, timeout)?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| request_error(format!("failed to configure read timeout: {error}")))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| request_error(format!("failed to configure write timeout: {error}")))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|error| request_error(format!("failed to write metadata request: {error}")))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| request_error(format!("failed to read metadata response: {error}")))?;
    Ok(response)
}

fn connect(endpoint: &str, timeout: Duration) -> LambdaMetadataResult<TcpStream> {
    let addresses = endpoint
        .to_socket_addrs()
        .map_err(|error| request_error(format!("failed to resolve metadata endpoint: {error}")))?;
    let mut last_error = None;

    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }

    Err(request_error(last_error.map_or_else(
        || "metadata endpoint did not resolve to any socket address".to_owned(),
        |error| format!("failed to connect to metadata endpoint: {error}"),
    )))
}

fn request_error(message: String) -> LambdaMetadataError {
    LambdaMetadataError::new(LambdaMetadataErrorKind::Request, message)
}

fn parse_http_response(response: &[u8]) -> LambdaMetadataResult<Vec<u8>> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| {
            LambdaMetadataError::new(
                LambdaMetadataErrorKind::InvalidResponse,
                "metadata response is missing HTTP headers",
            )
        })?;
    let headers = std::str::from_utf8(&response[..header_end]).map_err(|error| {
        LambdaMetadataError::new(
            LambdaMetadataErrorKind::InvalidResponse,
            format!("metadata response headers are not UTF-8: {error}"),
        )
    })?;
    let mut lines = headers.split("\r\n");
    let status_line = lines.next().ok_or_else(|| {
        LambdaMetadataError::new(
            LambdaMetadataErrorKind::InvalidResponse,
            "metadata response is missing a status line",
        )
    })?;
    let status_code = parse_status_code(status_line)?;
    if status_code != 200 {
        return Err(LambdaMetadataError::http_status(status_code));
    }

    let is_chunked = lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("transfer-encoding")
                && value
                    .split(',')
                    .any(|token| token.trim().eq_ignore_ascii_case("chunked"))
        })
    });
    let body = &response[(header_end + 4)..];
    if is_chunked {
        decode_chunked_body(body)
    } else {
        Ok(body.to_vec())
    }
}

fn parse_status_code(status_line: &str) -> LambdaMetadataResult<u16> {
    let mut parts = status_line.split_whitespace();
    let Some(version) = parts.next() else {
        return Err(invalid_status_line(status_line));
    };
    if !version.starts_with("HTTP/") {
        return Err(invalid_status_line(status_line));
    }

    let Some(status_code) = parts.next().and_then(|part| part.parse::<u16>().ok()) else {
        return Err(invalid_status_line(status_line));
    };
    Ok(status_code)
}

fn invalid_status_line(status_line: &str) -> LambdaMetadataError {
    LambdaMetadataError::new(
        LambdaMetadataErrorKind::InvalidResponse,
        format!("metadata response has invalid status line: {status_line}"),
    )
}

fn decode_chunked_body(body: &[u8]) -> LambdaMetadataResult<Vec<u8>> {
    let mut decoded = Vec::new();
    let mut position = 0;

    loop {
        let line_end = find_crlf(&body[position..]).ok_or_else(|| {
            LambdaMetadataError::new(
                LambdaMetadataErrorKind::InvalidResponse,
                "chunked metadata response is missing a chunk size",
            )
        })?;
        let size_line =
            std::str::from_utf8(&body[position..(position + line_end)]).map_err(|error| {
                LambdaMetadataError::new(
                    LambdaMetadataErrorKind::InvalidResponse,
                    format!("chunk size is not UTF-8: {error}"),
                )
            })?;
        let size_text = size_line.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|error| {
            LambdaMetadataError::new(
                LambdaMetadataErrorKind::InvalidResponse,
                format!("chunk size is invalid: {error}"),
            )
        })?;
        position += line_end + 2;

        if size == 0 {
            return Ok(decoded);
        }

        if body.len() < position + size + 2 {
            return Err(LambdaMetadataError::new(
                LambdaMetadataErrorKind::InvalidResponse,
                "chunked metadata response ended before the declared chunk length",
            ));
        }
        decoded.extend_from_slice(&body[position..(position + size)]);
        position += size;

        if body.get(position..(position + 2)) != Some(b"\r\n") {
            return Err(LambdaMetadataError::new(
                LambdaMetadataErrorKind::InvalidResponse,
                "chunked metadata response is missing a chunk terminator",
            ));
        }
        position += 2;
    }
}

fn find_crlf(bytes: &[u8]) -> Option<usize> {
    bytes.windows(2).position(|window| window == b"\r\n")
}

fn metadata_from_body(body: &[u8]) -> LambdaMetadataResult<LambdaMetadata> {
    let value = serde_json::from_slice::<Value>(body).map_err(|error| {
        LambdaMetadataError::new(
            LambdaMetadataErrorKind::Parse,
            format!("failed to parse metadata response: {error}"),
        )
    })?;
    let Value::Object(raw) = value else {
        return Err(LambdaMetadataError::new(
            LambdaMetadataErrorKind::Parse,
            "metadata response must be a JSON object",
        ));
    };

    Ok(LambdaMetadata::from_raw(raw))
}

fn metadata_cache() -> &'static Mutex<Option<LambdaMetadata>> {
    METADATA_CACHE.get_or_init(|| Mutex::new(None))
}

fn lock_metadata_cache() -> MutexGuard<'static, Option<LambdaMetadata>> {
    metadata_cache()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

    use serde_json::{Map, Value};
    use victors_lambdas_core::env;

    use super::{
        LambdaMetadata, LambdaMetadataClient, LambdaMetadataErrorKind, clear_lambda_metadata_cache,
    };

    #[test]
    fn metadata_from_raw_extracts_availability_zone_id() {
        let raw = Map::from_iter([
            (
                "AvailabilityZoneID".to_owned(),
                Value::String("use1-az1".to_owned()),
            ),
            ("FutureField".to_owned(), Value::Bool(true)),
        ]);

        let metadata = LambdaMetadata::from_raw(raw);

        assert_eq!(metadata.availability_zone_id(), Some("use1-az1"));
        assert_eq!(metadata.raw().get("FutureField"), Some(&Value::Bool(true)));
        assert!(!metadata.is_empty());
    }

    #[test]
    fn fetch_with_env_returns_empty_outside_lambda_or_in_dev_mode() {
        let client = LambdaMetadataClient::new();

        let local = client.fetch_with_env(|_| None).expect("local fetch");
        let dev = client
            .fetch_with_env(|name| {
                (name == env::POWERTOOLS_DEV)
                    .then(|| "true".to_owned())
                    .or_else(|| {
                        (name == env::AWS_LAMBDA_INITIALIZATION_TYPE)
                            .then(|| "on-demand".to_owned())
                    })
            })
            .expect("dev fetch");

        assert!(local.is_empty());
        assert!(dev.is_empty());
    }

    #[test]
    fn fetch_with_env_requires_metadata_env_in_lambda() {
        let error = LambdaMetadataClient::new()
            .fetch_with_env(|name| {
                (name == env::AWS_LAMBDA_INITIALIZATION_TYPE).then(|| "on-demand".to_owned())
            })
            .expect_err("missing endpoint should fail");

        assert_eq!(error.kind(), LambdaMetadataErrorKind::MissingEnvironment);
        assert!(error.message().contains(env::AWS_LAMBDA_METADATA_API));
    }

    #[test]
    fn fetch_from_endpoint_loads_metadata_and_sends_authorization() {
        let body = r#"{"AvailabilityZoneID":"use1-az1","FutureField":true}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let (endpoint, request) = metadata_server(response);

        let metadata = LambdaMetadataClient::new()
            .fetch_from_endpoint(&endpoint, "token-1")
            .expect("metadata fetch");
        let request = request.join().expect("server thread");

        assert_eq!(metadata.availability_zone_id(), Some("use1-az1"));
        assert_eq!(metadata.raw().get("FutureField"), Some(&Value::Bool(true)));
        assert!(request.starts_with("GET /2026-01-15/metadata/execution-environment HTTP/1.1"));
        assert!(request.contains("Authorization: Bearer token-1"));
    }

    #[test]
    fn fetch_from_endpoint_decodes_chunked_metadata() {
        let body = r#"{"AvailabilityZoneID":"az-1"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:X}\r\n{body}\r\n0\r\n\r\n",
            body.len()
        );
        let (endpoint, request) = metadata_server(response);

        let metadata = LambdaMetadataClient::new()
            .fetch_from_endpoint(&endpoint, "token-1")
            .expect("metadata fetch");
        let _request = request.join().expect("server thread");

        assert_eq!(metadata.availability_zone_id(), Some("az-1"));
    }

    #[test]
    fn fetch_from_endpoint_reports_http_status() {
        let (endpoint, request) = metadata_server(
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n".to_owned(),
        );

        let error = LambdaMetadataClient::new()
            .fetch_from_endpoint(&endpoint, "token-1")
            .expect_err("non-200 should fail");
        let _request = request.join().expect("server thread");

        assert_eq!(error.kind(), LambdaMetadataErrorKind::HttpStatus);
        assert_eq!(error.status_code(), Some(503));
    }

    #[test]
    fn fetch_from_endpoint_reports_invalid_json() {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nnot-json".to_owned();
        let (endpoint, request) = metadata_server(response);

        let error = LambdaMetadataClient::new()
            .fetch_from_endpoint(&endpoint, "token-1")
            .expect_err("invalid json should fail");
        let _request = request.join().expect("server thread");

        assert_eq!(error.kind(), LambdaMetadataErrorKind::Parse);
    }

    #[test]
    fn clear_metadata_cache_reports_existing_cached_value() {
        assert!(!clear_lambda_metadata_cache());
    }

    fn metadata_server(response: String) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind server");
        let endpoint = listener.local_addr().expect("local addr").to_string();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .expect("read timeout");
            let mut buffer = [0_u8; 2048];
            let read = stream.read(&mut buffer).expect("read request");
            stream
                .write_all(response.as_bytes())
                .expect("write response");
            String::from_utf8_lossy(&buffer[..read]).into_owned()
        });

        (endpoint, handle)
    }
}
