//! Field path normalization.

use crate::{DataMaskingError, DataMaskingResult};

pub(crate) fn to_json_pointer(path: &str) -> DataMaskingResult<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(DataMaskingError::invalid_path(path));
    }

    if trimmed.starts_with('/') {
        return Ok(trimmed.to_string());
    }

    let mut pointer = String::new();
    for segment in trimmed.split('.') {
        if segment.is_empty() {
            return Err(DataMaskingError::invalid_path(path));
        }

        pointer.push('/');
        pointer.push_str(&escape_pointer_segment(segment));
    }

    Ok(pointer)
}

fn escape_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_json_pointer_paths() {
        let pointer = to_json_pointer("/customer/password").expect("path should parse");

        assert_eq!(pointer, "/customer/password");
    }

    #[test]
    fn converts_dot_paths() {
        let pointer = to_json_pointer("customer.password").expect("path should parse");

        assert_eq!(pointer, "/customer/password");
    }

    #[test]
    fn escapes_dot_path_segments() {
        let pointer = to_json_pointer("customer.api/key~1").expect("path should parse");

        assert_eq!(pointer, "/customer/api~1key~01");
    }

    #[test]
    fn rejects_empty_paths() {
        let error = to_json_pointer(" ").expect_err("path should fail");

        assert_eq!(error.kind(), crate::DataMaskingErrorKind::InvalidPath);
    }
}
