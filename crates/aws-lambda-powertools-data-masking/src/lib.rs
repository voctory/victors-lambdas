//! Data masking utility.

mod error;
mod mask;
mod masking;
mod path;

pub use error::{DataMaskingError, DataMaskingErrorKind, DataMaskingResult};
pub use mask::{DATA_MASKING_STRING, MaskingOptions, MaskingStrategy};
pub use masking::{DataMasking, DataMaskingConfig, erase, erase_fields};
