//! Data masking utility.

mod error;
mod mask;
mod masking;
mod path;
mod provider;

pub use error::{DataMaskingError, DataMaskingErrorKind, DataMaskingResult};
pub use mask::{DATA_MASKING_STRING, MaskingOptions, MaskingStrategy};
pub use masking::{DataMasking, DataMaskingConfig, erase, erase_fields};
#[cfg(feature = "kms")]
pub use provider::KmsDataMaskingProvider;
pub use provider::{DataMaskingProvider, EncryptionContext};
