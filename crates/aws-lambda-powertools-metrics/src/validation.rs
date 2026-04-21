//! `CloudWatch` EMF validation helpers.

use crate::MetricsError;

const MAX_MEMBER_NAME_LENGTH: usize = 255;
const RESERVED_AWS_MEMBER: &str = "_aws";

pub(crate) fn validate_metric_name(name: &str) -> Result<(), MetricsError> {
    validate_member_name(name).map_err(|reason| MetricsError::InvalidMetricName {
        name: name.to_owned(),
        reason,
    })
}

pub(crate) fn validate_dimension_name(name: &str) -> Result<(), MetricsError> {
    validate_member_name(name).map_err(|reason| MetricsError::InvalidDimensionName {
        name: name.to_owned(),
        reason,
    })?;

    if name.starts_with(':') {
        return Err(MetricsError::InvalidDimensionName {
            name: name.to_owned(),
            reason: "must not start with ':'",
        });
    }

    Ok(())
}

pub(crate) fn validate_metadata_name(name: &str) -> Result<(), MetricsError> {
    validate_member_name(name).map_err(|reason| MetricsError::InvalidMetadataName {
        name: name.to_owned(),
        reason,
    })
}

fn validate_member_name(name: &str) -> Result<(), &'static str> {
    if name.trim().is_empty() {
        return Err("must contain at least one non-whitespace character");
    }

    if name.len() > MAX_MEMBER_NAME_LENGTH {
        return Err("must be 255 bytes or fewer");
    }

    if name == RESERVED_AWS_MEMBER {
        return Err("is reserved for EMF metadata");
    }

    if !name.is_ascii() {
        return Err("must contain only ASCII characters");
    }

    if name.chars().any(char::is_control) {
        return Err("must not contain control characters");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_names_allow_plain_ascii_names() {
        assert_eq!(validate_metric_name("Processed"), Ok(()));
        assert_eq!(validate_dimension_name("Operation"), Ok(()));
        assert_eq!(validate_metadata_name("request_id"), Ok(()));
    }

    #[test]
    fn member_names_reject_empty_reserved_non_ascii_and_control_names() {
        assert_eq!(
            validate_metric_name("   "),
            Err(MetricsError::InvalidMetricName {
                name: "   ".to_owned(),
                reason: "must contain at least one non-whitespace character"
            })
        );
        assert_eq!(
            validate_metadata_name("_aws"),
            Err(MetricsError::InvalidMetadataName {
                name: "_aws".to_owned(),
                reason: "is reserved for EMF metadata"
            })
        );
        assert_eq!(
            validate_metric_name("Metríc"),
            Err(MetricsError::InvalidMetricName {
                name: "Metríc".to_owned(),
                reason: "must contain only ASCII characters"
            })
        );
        assert_eq!(
            validate_metric_name("Metric\nName"),
            Err(MetricsError::InvalidMetricName {
                name: "Metric\nName".to_owned(),
                reason: "must not contain control characters"
            })
        );
    }

    #[test]
    fn dimension_names_reject_colon_prefix() {
        assert_eq!(
            validate_dimension_name(":Operation"),
            Err(MetricsError::InvalidDimensionName {
                name: ":Operation".to_owned(),
                reason: "must not start with ':'"
            })
        );
    }

    #[test]
    fn member_names_reject_values_longer_than_255_bytes() {
        let name = "a".repeat(MAX_MEMBER_NAME_LENGTH + 1);

        assert_eq!(
            validate_metric_name(&name),
            Err(MetricsError::InvalidMetricName {
                name,
                reason: "must be 255 bytes or fewer"
            })
        );
    }
}
