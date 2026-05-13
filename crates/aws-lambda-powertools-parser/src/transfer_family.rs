//! AWS Transfer Family authorizer event models.

use std::{fmt, net::IpAddr};

use serde::{Deserialize, Serialize, Serializer};

/// AWS Transfer Family protocol for a custom identity provider request.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum TransferFamilyProtocol {
    /// Secure Shell File Transfer Protocol.
    #[serde(rename = "SFTP")]
    Sftp,
    /// File Transfer Protocol.
    #[serde(rename = "FTP")]
    Ftp,
    /// File Transfer Protocol over TLS.
    #[serde(rename = "FTPS")]
    Ftps,
}

/// AWS Transfer Family custom identity provider authorizer event.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferFamilyAuthorizerEvent {
    /// Username attempting to authenticate.
    pub username: String,
    /// Password supplied for password authentication.
    ///
    /// This is absent when a client authenticates with SSH public keys.
    #[serde(default)]
    pub password: Option<String>,
    /// Transfer protocol used by the client.
    pub protocol: TransferFamilyProtocol,
    /// AWS Transfer Family server ID.
    pub server_id: String,
    /// Source IP address of the connecting client.
    pub source_ip: IpAddr,
}

/// AWS Transfer Family home directory mapping mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum TransferFamilyHomeDirectoryType {
    /// Use a direct path as the user's home directory.
    #[serde(rename = "PATH")]
    Path,
    /// Use logical directory mappings.
    #[serde(rename = "LOGICAL")]
    Logical,
}

/// AWS Transfer Family logical home directory entry.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TransferFamilyHomeDirectoryEntry {
    /// Virtual path exposed to the transfer user.
    pub entry: String,
    /// S3 or EFS target path backing the virtual entry.
    pub target: String,
}

impl TransferFamilyHomeDirectoryEntry {
    /// Creates a logical home directory mapping entry.
    #[must_use]
    pub fn new(entry: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            entry: entry.into(),
            target: target.into(),
        }
    }
}

/// AWS Transfer Family POSIX profile for EFS-backed users.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TransferFamilyPosixProfile {
    /// POSIX user ID.
    pub uid: u32,
    /// POSIX group ID.
    pub gid: u32,
    /// Optional secondary POSIX group IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_gids: Vec<u32>,
}

impl TransferFamilyPosixProfile {
    /// Creates a POSIX profile with no secondary groups.
    #[must_use]
    pub const fn new(uid: u32, gid: u32) -> Self {
        Self {
            uid,
            gid,
            secondary_gids: Vec::new(),
        }
    }

    /// Returns a copy with secondary POSIX group IDs.
    #[must_use]
    pub fn with_secondary_gids(mut self, secondary_gids: impl IntoIterator<Item = u32>) -> Self {
        self.secondary_gids = secondary_gids.into_iter().collect();
        self
    }
}

/// Error returned when building an invalid Transfer Family authorizer response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferFamilyResponseError {
    /// The IAM role ARN is empty.
    EmptyRole,
    /// A path response was requested without a home directory.
    EmptyHomeDirectory,
    /// A logical response was requested without home directory mappings.
    EmptyHomeDirectoryDetails,
}

impl fmt::Display for TransferFamilyResponseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRole => formatter.write_str("Transfer Family role ARN cannot be empty"),
            Self::EmptyHomeDirectory => {
                formatter.write_str("Transfer Family home directory cannot be empty")
            }
            Self::EmptyHomeDirectoryDetails => formatter
                .write_str("Transfer Family logical home directory details cannot be empty"),
        }
    }
}

impl std::error::Error for TransferFamilyResponseError {}

/// Result returned by Transfer Family response builders.
pub type TransferFamilyResponseResult<T> = Result<T, TransferFamilyResponseError>;

/// AWS Transfer Family custom identity provider authorizer response.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TransferFamilyAuthorizerResponse {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    home_directory: Option<String>,
    home_directory_type: TransferFamilyHomeDirectoryType,
    #[serde(
        serialize_with = "serialize_home_directory_details",
        skip_serializing_if = "Vec::is_empty"
    )]
    home_directory_details: Vec<TransferFamilyHomeDirectoryEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    posix_profile: Option<TransferFamilyPosixProfile>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    public_keys: Vec<String>,
}

impl TransferFamilyAuthorizerResponse {
    /// Creates a response that uses a direct home directory path.
    ///
    /// # Errors
    ///
    /// Returns [`TransferFamilyResponseError`] when `role` or
    /// `home_directory` is empty.
    pub fn path(
        role: impl Into<String>,
        home_directory: impl Into<String>,
    ) -> TransferFamilyResponseResult<Self> {
        let role = role.into();
        if role.is_empty() {
            return Err(TransferFamilyResponseError::EmptyRole);
        }

        let home_directory = home_directory.into();
        if home_directory.is_empty() {
            return Err(TransferFamilyResponseError::EmptyHomeDirectory);
        }

        Ok(Self {
            role,
            policy: None,
            home_directory: Some(home_directory),
            home_directory_type: TransferFamilyHomeDirectoryType::Path,
            home_directory_details: Vec::new(),
            posix_profile: None,
            public_keys: Vec::new(),
        })
    }

    /// Creates a response that uses logical home directory mappings.
    ///
    /// # Errors
    ///
    /// Returns [`TransferFamilyResponseError`] when `role` is empty or no
    /// mappings are supplied.
    pub fn logical(
        role: impl Into<String>,
        home_directory_details: impl IntoIterator<Item = TransferFamilyHomeDirectoryEntry>,
    ) -> TransferFamilyResponseResult<Self> {
        let role = role.into();
        if role.is_empty() {
            return Err(TransferFamilyResponseError::EmptyRole);
        }

        let home_directory_details = home_directory_details.into_iter().collect::<Vec<_>>();
        if home_directory_details.is_empty() {
            return Err(TransferFamilyResponseError::EmptyHomeDirectoryDetails);
        }

        Ok(Self {
            role,
            policy: None,
            home_directory: None,
            home_directory_type: TransferFamilyHomeDirectoryType::Logical,
            home_directory_details,
            posix_profile: None,
            public_keys: Vec::new(),
        })
    }

    /// Adds an IAM session policy document to the response.
    #[must_use]
    pub fn with_policy(mut self, policy: impl Into<String>) -> Self {
        self.policy = Some(policy.into());
        self
    }

    /// Adds a POSIX profile to the response for EFS-backed users.
    #[must_use]
    pub fn with_posix_profile(mut self, posix_profile: TransferFamilyPosixProfile) -> Self {
        self.posix_profile = Some(posix_profile);
        self
    }

    /// Adds SSH public keys to the response.
    #[must_use]
    pub fn with_public_keys<I, K>(mut self, public_keys: I) -> Self
    where
        I: IntoIterator<Item = K>,
        K: Into<String>,
    {
        self.public_keys = public_keys.into_iter().map(Into::into).collect();
        self
    }
}

fn serialize_home_directory_details<S>(
    details: &[TransferFamilyHomeDirectoryEntry],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let encoded = serde_json::to_string(details).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&encoded)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use serde_json::{Value, json};

    use super::{
        TransferFamilyAuthorizerEvent, TransferFamilyAuthorizerResponse,
        TransferFamilyHomeDirectoryEntry, TransferFamilyPosixProfile, TransferFamilyProtocol,
        TransferFamilyResponseError,
    };
    use crate::{EventParser, ParseErrorKind};

    #[test]
    fn parses_transfer_family_authorizer_event() {
        let parsed = EventParser::new()
            .parse_json_value::<TransferFamilyAuthorizerEvent>(json!({
                "username": "test-user",
                "password": "test-pass",
                "protocol": "SFTP",
                "serverId": "s-abcd123456",
                "sourceIp": "192.168.0.100"
            }))
            .expect("Transfer Family event should parse");

        assert_eq!(parsed.payload().username, "test-user");
        assert_eq!(parsed.payload().password.as_deref(), Some("test-pass"));
        assert_eq!(parsed.payload().protocol, TransferFamilyProtocol::Sftp);
        assert_eq!(
            parsed.payload().source_ip,
            IpAddr::V4(Ipv4Addr::new(192, 168, 0, 100))
        );
    }

    #[test]
    fn parses_transfer_family_authorizer_event_without_password() {
        let parsed = EventParser::new()
            .parse_json_value::<TransferFamilyAuthorizerEvent>(json!({
                "username": "test-user",
                "protocol": "FTPS",
                "serverId": "s-abcd123456",
                "sourceIp": "2001:db8::1"
            }))
            .expect("Transfer Family event should parse without password");

        assert_eq!(parsed.payload().password, None);
        assert_eq!(parsed.payload().protocol, TransferFamilyProtocol::Ftps);
    }

    #[test]
    fn rejects_invalid_transfer_family_authorizer_event() {
        let error = EventParser::new()
            .parse_json_value::<TransferFamilyAuthorizerEvent>(json!({
                "username": "test-user",
                "protocol": "SFTP",
                "serverId": "s-abcd123456",
                "sourceIp": "invalid-ip"
            }))
            .expect_err("invalid IP should fail");

        assert_eq!(error.kind(), ParseErrorKind::Data);
        assert!(error.message().contains("invalid IP address syntax"));
    }

    #[test]
    fn builds_transfer_family_path_response() {
        let response = TransferFamilyAuthorizerResponse::path(
            "arn:aws:iam::123456789012:role/S3Access",
            "/bucket/user",
        )
        .expect("path response should build")
        .with_policy(r#"{"Version":"2012-10-17"}"#)
        .with_public_keys(["ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC0"]);

        let value = serde_json::to_value(response).expect("response should serialize");

        assert_eq!(
            value,
            json!({
                "Role": "arn:aws:iam::123456789012:role/S3Access",
                "Policy": r#"{"Version":"2012-10-17"}"#,
                "HomeDirectory": "/bucket/user",
                "HomeDirectoryType": "PATH",
                "PublicKeys": ["ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC0"]
            })
        );
    }

    #[test]
    fn builds_transfer_family_logical_response() {
        let response = TransferFamilyAuthorizerResponse::logical(
            "arn:aws:iam::123456789012:role/EfsAccess",
            [TransferFamilyHomeDirectoryEntry::new(
                "/",
                "/bucket/${transfer:UserName}",
            )],
        )
        .expect("logical response should build")
        .with_posix_profile(
            TransferFamilyPosixProfile::new(1000, 1000).with_secondary_gids([1001]),
        );

        let value = serde_json::to_value(response).expect("response should serialize");

        assert_eq!(
            value,
            json!({
                "Role": "arn:aws:iam::123456789012:role/EfsAccess",
                "HomeDirectoryType": "LOGICAL",
                "HomeDirectoryDetails": r#"[{"Entry":"/","Target":"/bucket/${transfer:UserName}"}]"#,
                "PosixProfile": {
                    "Uid": 1000,
                    "Gid": 1000,
                    "SecondaryGids": [1001]
                }
            })
        );
    }

    #[test]
    fn rejects_invalid_transfer_family_responses() {
        let empty_details: [TransferFamilyHomeDirectoryEntry; 0] = [];

        assert_eq!(
            TransferFamilyAuthorizerResponse::path("", "/bucket/user"),
            Err(TransferFamilyResponseError::EmptyRole)
        );
        assert_eq!(
            TransferFamilyAuthorizerResponse::path("arn:aws:iam::123456789012:role/S3Access", ""),
            Err(TransferFamilyResponseError::EmptyHomeDirectory)
        );
        assert_eq!(
            TransferFamilyAuthorizerResponse::logical(
                "arn:aws:iam::123456789012:role/S3Access",
                empty_details
            ),
            Err(TransferFamilyResponseError::EmptyHomeDirectoryDetails)
        );
    }

    #[test]
    fn transfer_family_response_details_are_serialized_as_a_json_string() {
        let response = TransferFamilyAuthorizerResponse::logical(
            "arn:aws:iam::123456789012:role/S3Access",
            [TransferFamilyHomeDirectoryEntry::new("/", "/bucket/user")],
        )
        .expect("logical response should build");

        let value = serde_json::to_value(response).expect("response should serialize");

        assert!(matches!(value["HomeDirectoryDetails"], Value::String(_)));
    }
}
