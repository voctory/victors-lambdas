//! Encryption and decryption providers for data masking.

use std::collections::HashMap;

#[cfg(feature = "kms")]
use std::future::Future;

#[cfg(feature = "kms")]
use aws_sdk_kms::{Client as AwsKmsClient, primitives::Blob};
use base64::{Engine as _, engine::general_purpose::STANDARD};

#[cfg(feature = "kms")]
use crate::DataMaskingErrorKind;
use crate::{DataMaskingError, DataMaskingResult};

/// Encryption context values authenticated with encrypted data.
pub type EncryptionContext = HashMap<String, String>;

/// Provider abstraction used by [`crate::DataMasking`] encryption and decryption helpers.
pub trait DataMaskingProvider {
    /// Encrypts serialized plaintext bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider cannot encrypt the plaintext.
    fn encrypt(
        &mut self,
        plaintext: &[u8],
        encryption_context: &EncryptionContext,
    ) -> DataMaskingResult<Vec<u8>>;

    /// Decrypts ciphertext bytes into serialized plaintext bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider cannot decrypt the ciphertext.
    fn decrypt(
        &mut self,
        ciphertext: &[u8],
        encryption_context: &EncryptionContext,
    ) -> DataMaskingResult<Vec<u8>>;
}

pub(crate) fn encode_ciphertext(ciphertext: &[u8]) -> String {
    STANDARD.encode(ciphertext)
}

pub(crate) fn decode_ciphertext(ciphertext: &str) -> DataMaskingResult<Vec<u8>> {
    STANDARD
        .decode(ciphertext)
        .map_err(DataMaskingError::decrypt)
}

/// Direct AWS KMS provider for data masking encryption and decryption.
///
/// This provider calls the KMS `Encrypt` and `Decrypt` APIs directly. It is useful for small JSON
/// payloads that fit KMS request limits. It is not an AWS Encryption SDK envelope encryption
/// provider and does not implement cached data keys.
#[cfg(feature = "kms")]
#[derive(Clone, Debug)]
pub struct KmsDataMaskingProvider {
    client: AwsKmsClient,
    key_id: String,
}

#[cfg(feature = "kms")]
impl KmsDataMaskingProvider {
    /// Creates a direct AWS KMS data masking provider.
    #[must_use]
    pub fn new(client: AwsKmsClient, key_id: impl Into<String>) -> Self {
        Self {
            client,
            key_id: key_id.into(),
        }
    }

    /// Returns the underlying AWS SDK KMS client.
    #[must_use]
    pub const fn client(&self) -> &AwsKmsClient {
        &self.client
    }

    /// Returns the configured KMS key ID or ARN.
    #[must_use]
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

#[cfg(feature = "kms")]
impl DataMaskingProvider for KmsDataMaskingProvider {
    fn encrypt(
        &mut self,
        plaintext: &[u8],
        encryption_context: &EncryptionContext,
    ) -> DataMaskingResult<Vec<u8>> {
        let client = self.client.clone();
        let key_id = self.key_id.clone();
        let plaintext = plaintext.to_vec();
        let encryption_context = encryption_context.clone();

        run_kms_request(
            async move {
                let output = client
                    .encrypt()
                    .key_id(key_id)
                    .plaintext(Blob::new(plaintext))
                    .set_encryption_context(Some(encryption_context))
                    .send()
                    .await
                    .map_err(DataMaskingError::encrypt)?;

                output
                    .ciphertext_blob()
                    .map(|blob| blob.as_ref().to_vec())
                    .ok_or_else(|| {
                        DataMaskingError::encrypt("KMS Encrypt response did not include ciphertext")
                    })
            },
            DataMaskingErrorKind::Encrypt,
        )
    }

    fn decrypt(
        &mut self,
        ciphertext: &[u8],
        encryption_context: &EncryptionContext,
    ) -> DataMaskingResult<Vec<u8>> {
        let client = self.client.clone();
        let ciphertext = ciphertext.to_vec();
        let encryption_context = encryption_context.clone();

        run_kms_request(
            async move {
                let output = client
                    .decrypt()
                    .ciphertext_blob(Blob::new(ciphertext))
                    .set_encryption_context(Some(encryption_context))
                    .send()
                    .await
                    .map_err(DataMaskingError::decrypt)?;

                output
                    .plaintext()
                    .map(|blob| blob.as_ref().to_vec())
                    .ok_or_else(|| {
                        DataMaskingError::decrypt("KMS Decrypt response did not include plaintext")
                    })
            },
            DataMaskingErrorKind::Decrypt,
        )
    }
}

#[cfg(feature = "kms")]
fn run_kms_request<F, T>(future: F, error_kind: DataMaskingErrorKind) -> DataMaskingResult<T>
where
    F: Future<Output = DataMaskingResult<T>> + Send + 'static,
    T: Send + 'static,
{
    let worker = std::thread::Builder::new()
        .name("powertools-data-masking-kms".to_owned())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| provider_error(error_kind, error))?;
            runtime.block_on(future)
        })
        .map_err(|error| provider_error(error_kind, error))?;

    worker
        .join()
        .map_err(|_| provider_error(error_kind, "KMS worker thread panicked"))?
}

#[cfg(feature = "kms")]
fn provider_error(
    error_kind: DataMaskingErrorKind,
    error: impl std::fmt::Display,
) -> DataMaskingError {
    if error_kind == DataMaskingErrorKind::Decrypt {
        DataMaskingError::decrypt(error)
    } else {
        DataMaskingError::encrypt(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ciphertext_codec_round_trips_bytes() {
        let encoded = encode_ciphertext(b"ciphertext");
        let decoded = decode_ciphertext(&encoded).expect("base64 should decode");

        assert_eq!(decoded, b"ciphertext");
    }

    #[test]
    fn ciphertext_codec_rejects_invalid_base64() {
        let error = decode_ciphertext("not base64").expect_err("decode should fail");

        assert_eq!(error.kind(), crate::DataMaskingErrorKind::Decrypt);
    }

    #[cfg(feature = "kms")]
    #[test]
    fn kms_provider_keeps_configured_client_and_key() {
        let client = aws_kms_client();
        let provider = KmsDataMaskingProvider::new(client, "alias/data-masking");

        assert!(provider.client().config().region().is_some());
        assert_eq!(provider.key_id(), "alias/data-masking");
    }

    #[cfg(feature = "kms")]
    fn aws_kms_client() -> AwsKmsClient {
        use aws_sdk_kms::{
            Config,
            config::{BehaviorVersion, Credentials, Region},
        };

        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new(
                "access-key",
                "secret-key",
                None,
                None,
                "test",
            ))
            .build();

        AwsKmsClient::from_conf(config)
    }
}
