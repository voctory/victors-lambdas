//! `JMESPath` extraction utility.

mod envelope;
mod error;
mod expression;
mod functions;

pub use envelope::{
    API_GATEWAY_HTTP, API_GATEWAY_REST, CLOUDWATCH_EVENTS_SCHEDULED, CLOUDWATCH_LOGS, EVENTBRIDGE,
    KINESIS_DATA_STREAM, S3_EVENTBRIDGE_SQS, S3_KINESIS_FIREHOSE, S3_SNS_KINESIS_FIREHOSE,
    S3_SNS_SQS, S3_SQS, SNS, SQS, extract_data_from_envelope,
};
pub use error::{JmespathError, JmespathErrorKind, JmespathResult};
pub use expression::{JmespathExpression, query, search, search_as};
