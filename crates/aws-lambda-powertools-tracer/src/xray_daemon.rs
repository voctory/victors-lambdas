//! AWS X-Ray daemon UDP transport.

use std::{
    error::Error,
    fmt, io,
    net::{ToSocketAddrs, UdpSocket},
};

use aws_lambda_powertools_core::env;

use crate::{TraceSegment, XrayDocumentError};

const DEFAULT_XRAY_DAEMON_ADDRESS: &str = "127.0.0.1:2000";
const XRAY_DAEMON_PACKET_HEADER: &str = "{\"format\":\"json\",\"version\":1}\n";

/// Configuration for sending X-Ray segment documents to the local daemon.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XrayDaemonConfig {
    address: String,
}

impl XrayDaemonConfig {
    /// Creates configuration for a daemon address.
    ///
    /// Empty addresses fall back to `127.0.0.1:2000`. Values using the X-Ray
    /// SDK format, such as `tcp:127.0.0.1:2000 udp:127.0.0.1:2000`, prefer the
    /// UDP endpoint.
    #[must_use]
    pub fn new(address: impl AsRef<str>) -> Self {
        Self {
            address: normalize_daemon_address(address.as_ref())
                .unwrap_or_else(|| DEFAULT_XRAY_DAEMON_ADDRESS.to_owned()),
        }
    }

    /// Creates configuration from `AWS_XRAY_DAEMON_ADDRESS`.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(env::var_or(
            env::AWS_XRAY_DAEMON_ADDRESS,
            DEFAULT_XRAY_DAEMON_ADDRESS,
        ))
    }

    /// Returns the daemon address used for UDP sends.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }
}

impl Default for XrayDaemonConfig {
    fn default() -> Self {
        Self::new(DEFAULT_XRAY_DAEMON_ADDRESS)
    }
}

/// Error returned when a segment cannot be sent to the X-Ray daemon.
#[derive(Debug)]
pub enum XrayDaemonError {
    /// The segment could not be rendered as an X-Ray document.
    Document(XrayDocumentError),
    /// The UDP packet could not be sent.
    Io(io::Error),
}

impl fmt::Display for XrayDaemonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(error) => write!(formatter, "X-Ray document error: {error}"),
            Self::Io(error) => write!(formatter, "X-Ray daemon transport error: {error}"),
        }
    }
}

impl Error for XrayDaemonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Document(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<XrayDocumentError> for XrayDaemonError {
    fn from(error: XrayDocumentError) -> Self {
        Self::Document(error)
    }
}

impl From<io::Error> for XrayDaemonError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// UDP client for the AWS X-Ray daemon.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XrayDaemonClient {
    config: XrayDaemonConfig,
}

impl XrayDaemonClient {
    /// Creates a client with explicit configuration.
    #[must_use]
    pub fn new(config: XrayDaemonConfig) -> Self {
        Self { config }
    }

    /// Creates a client from `AWS_XRAY_DAEMON_ADDRESS`.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(XrayDaemonConfig::from_env())
    }

    /// Returns the configured daemon address.
    #[must_use]
    pub fn address(&self) -> &str {
        self.config.address()
    }

    /// Sends a pre-rendered X-Ray document to the daemon over UDP.
    ///
    /// The document is wrapped in the X-Ray daemon JSON packet header before it
    /// is sent.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the UDP socket cannot be created or the packet
    /// cannot be sent to the configured daemon address.
    pub fn send_document(&self, document: &str) -> io::Result<usize> {
        let address = self.address().to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "X-Ray daemon address did not resolve",
            )
        })?;
        let bind_address = if address.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };
        let socket = UdpSocket::bind(bind_address)?;
        socket.send_to(xray_daemon_packet(document).as_bytes(), address)
    }

    /// Renders and sends a trace segment as an X-Ray subsegment document.
    ///
    /// # Errors
    ///
    /// Returns [`XrayDaemonError`] when the document cannot be rendered or the
    /// UDP packet cannot be sent.
    pub fn send_subsegment(
        &self,
        segment: &TraceSegment,
        id: impl Into<String>,
        start_time: f64,
        end_time: f64,
    ) -> Result<usize, XrayDaemonError> {
        let document = segment.to_xray_subsegment_document(id, start_time, end_time)?;
        self.send_document(&document).map_err(XrayDaemonError::from)
    }
}

impl Default for XrayDaemonClient {
    fn default() -> Self {
        Self::new(XrayDaemonConfig::default())
    }
}

fn xray_daemon_packet(document: &str) -> String {
    let mut packet = String::with_capacity(XRAY_DAEMON_PACKET_HEADER.len() + document.len());
    packet.push_str(XRAY_DAEMON_PACKET_HEADER);
    packet.push_str(document);
    packet
}

fn normalize_daemon_address(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    trimmed
        .split_whitespace()
        .find_map(|token| token.strip_prefix("udp:"))
        .and_then(non_empty)
        .or_else(|| {
            trimmed
                .split_whitespace()
                .next()
                .map(strip_known_scheme)
                .and_then(non_empty)
        })
}

fn strip_known_scheme(value: &str) -> &str {
    value
        .strip_prefix("udp:")
        .or_else(|| value.strip_prefix("tcp:"))
        .unwrap_or(value)
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use std::{
        net::UdpSocket,
        time::{Duration, Instant},
    };

    use crate::{
        TraceContext, TraceSegment, Tracer, TracerConfig, XrayDaemonClient, XrayDaemonConfig,
        XrayDaemonError, XrayDocumentError,
    };

    use super::{DEFAULT_XRAY_DAEMON_ADDRESS, XRAY_DAEMON_PACKET_HEADER, xray_daemon_packet};

    #[test]
    fn config_parses_xray_daemon_address() {
        assert_eq!(
            XrayDaemonConfig::new("tcp:10.0.0.1:2000 udp:10.0.0.2:3000").address(),
            "10.0.0.2:3000"
        );
        assert_eq!(
            XrayDaemonConfig::new("tcp:10.0.0.3:2000").address(),
            "10.0.0.3:2000"
        );
        assert_eq!(
            XrayDaemonConfig::new("  ").address(),
            DEFAULT_XRAY_DAEMON_ADDRESS
        );
    }

    #[test]
    fn send_document_wraps_document_in_xray_daemon_packet() {
        let receiver = UdpSocket::bind("127.0.0.1:0").expect("receiver binds");
        receiver
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("timeout is set");
        let client = XrayDaemonClient::new(XrayDaemonConfig::new(
            receiver
                .local_addr()
                .expect("receiver has address")
                .to_string(),
        ));

        let bytes = client
            .send_document("{\"name\":\"handler\"}")
            .expect("document is sent");

        assert!(bytes > "{\"name\":\"handler\"}".len());
        assert_eq!(
            receive_udp_payload(&receiver),
            xray_daemon_packet("{\"name\":\"handler\"}")
        );
    }

    #[test]
    fn send_subsegment_renders_document_before_sending() {
        let receiver = UdpSocket::bind("127.0.0.1:0").expect("receiver binds");
        receiver
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("timeout is set");
        let client = XrayDaemonClient::new(XrayDaemonConfig::new(
            receiver
                .local_addr()
                .expect("receiver has address")
                .to_string(),
        ));
        let tracer = Tracer::with_config(TracerConfig::new("orders"));
        let context = TraceContext::new("handler")
            .with_trace_id("1-67891233-abcdef012345678912345678")
            .with_parent_id("53995c3f42cd8ad8");
        let segment = tracer
            .segment_with_context(context)
            .with_annotation("tenant", "north");

        client
            .send_subsegment(&segment, "70de5b6f19ff9a0a", 1.0, 2.0)
            .expect("subsegment is sent");

        let payload = receive_udp_payload(&receiver);
        assert!(payload.starts_with(XRAY_DAEMON_PACKET_HEADER));
        assert!(payload.contains("\"name\":\"handler\""));
        assert!(payload.contains("\"annotations\":{\"tenant\":\"north\"}"));
    }

    #[test]
    fn send_subsegment_returns_document_errors() {
        let segment = TraceSegment::new(TraceContext::new("handler"));
        let client = XrayDaemonClient::default();

        let error = client
            .send_subsegment(&segment, "70de5b6f19ff9a0a", 1.0, 2.0)
            .expect_err("missing trace context fails before UDP send");

        assert!(matches!(
            error,
            XrayDaemonError::Document(XrayDocumentError::MissingTraceId)
        ));
    }

    fn receive_udp_payload(receiver: &UdpSocket) -> String {
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut buffer = [0_u8; 4096];
        loop {
            match receiver.recv_from(&mut buffer) {
                Ok((len, _addr)) => {
                    return String::from_utf8(buffer[..len].to_vec()).expect("payload is utf8");
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "timed out waiting for UDP packet"
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    panic!("timed out waiting for UDP packet");
                }
                Err(error) => panic!("unexpected UDP receive error: {error}"),
            }
        }
    }
}
