//! Streaming test doubles.

use std::{collections::BTreeMap, io::Cursor};

use aws_lambda_powertools_streaming::{
    S3GetObjectRangeRequest, S3HeadObjectOutput, S3HeadObjectRequest, S3ObjectClient,
    S3ObjectIdentifier, StreamingError, StreamingErrorKind, StreamingResult,
};

/// In-memory S3 object client for testing streaming code.
///
/// The stub implements [`S3ObjectClient`] and records head-object and range
/// requests so tests can assert how a seekable S3 reader accessed an object.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct S3ObjectClientStub {
    objects: BTreeMap<S3ObjectKey, Vec<u8>>,
    head_requests: Vec<S3HeadObjectRequest>,
    range_requests: Vec<S3GetObjectRangeRequest>,
}

impl S3ObjectClientStub {
    /// Creates an empty S3 object client stub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an unversioned object and returns the updated stub.
    #[must_use]
    pub fn with_object(
        mut self,
        bucket: impl Into<String>,
        key: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        self.put_object(bucket, key, data);
        self
    }

    /// Adds a versioned object and returns the updated stub.
    #[must_use]
    pub fn with_versioned_object(
        mut self,
        bucket: impl Into<String>,
        key: impl Into<String>,
        version_id: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> Self {
        self.put_versioned_object(bucket, key, version_id, data);
        self
    }

    /// Adds or replaces an unversioned object.
    pub fn put_object(
        &mut self,
        bucket: impl Into<String>,
        key: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> &mut Self {
        self.objects
            .insert(S3ObjectKey::new(bucket, key, None), data.into());
        self
    }

    /// Adds or replaces a versioned object.
    pub fn put_versioned_object(
        &mut self,
        bucket: impl Into<String>,
        key: impl Into<String>,
        version_id: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> &mut Self {
        self.objects.insert(
            S3ObjectKey::new(bucket, key, Some(version_id.into())),
            data.into(),
        );
        self
    }

    /// Returns recorded head-object requests.
    #[must_use]
    pub fn head_requests(&self) -> &[S3HeadObjectRequest] {
        &self.head_requests
    }

    /// Returns recorded get-object range requests.
    #[must_use]
    pub fn range_requests(&self) -> &[S3GetObjectRangeRequest] {
        &self.range_requests
    }

    fn object_data(&self, object: &S3ObjectIdentifier) -> StreamingResult<&[u8]> {
        let key = S3ObjectKey::from_identifier(object);
        self.objects
            .get(&key)
            .map(Vec::as_slice)
            .ok_or_else(|| missing_object_error(object))
    }
}

impl S3ObjectClient for S3ObjectClientStub {
    type Reader = Cursor<Vec<u8>>;

    fn head_object(&mut self, request: S3HeadObjectRequest) -> StreamingResult<S3HeadObjectOutput> {
        let content_length = self.object_data(request.object())?.len() as u64;
        self.head_requests.push(request);

        Ok(S3HeadObjectOutput::new(content_length))
    }

    fn get_object_range(
        &mut self,
        request: S3GetObjectRangeRequest,
    ) -> StreamingResult<Self::Reader> {
        let offset = usize::try_from(request.offset()).unwrap_or(usize::MAX);
        let data = self
            .object_data(request.object())?
            .get(offset..)
            .map_or_else(Vec::new, ToOwned::to_owned);
        self.range_requests.push(request);

        Ok(Cursor::new(data))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct S3ObjectKey {
    bucket: String,
    key: String,
    version_id: Option<String>,
}

impl S3ObjectKey {
    fn new(bucket: impl Into<String>, key: impl Into<String>, version_id: Option<String>) -> Self {
        Self {
            bucket: bucket.into(),
            key: key.into(),
            version_id,
        }
    }

    fn from_identifier(object: &S3ObjectIdentifier) -> Self {
        Self::new(
            object.bucket(),
            object.key(),
            object.version_id().map(ToOwned::to_owned),
        )
    }
}

fn missing_object_error(object: &S3ObjectIdentifier) -> StreamingError {
    let version = object
        .version_id()
        .map(|version_id| format!(" version {version_id}"))
        .unwrap_or_default();

    StreamingError::new(
        StreamingErrorKind::Io,
        format!(
            "S3 object stub has no object for bucket {} key {}{version}",
            object.bucket(),
            object.key()
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::io::{Read as _, Seek as _, SeekFrom};

    use aws_lambda_powertools_streaming::{
        S3HeadObjectRequest, S3Object, S3ObjectClient, S3ObjectIdentifier, StreamingErrorKind,
    };

    use super::S3ObjectClientStub;

    #[test]
    fn stub_serves_seekable_s3_object_ranges() {
        let client = S3ObjectClientStub::new().with_object("orders", "order.json", b"abcdef");
        let mut object = S3Object::for_bucket_key("orders", "order.json", client);

        assert_eq!(object.size().expect("object size should load"), 6);

        let mut first = [0; 2];
        object.read_exact(&mut first).expect("range should read");
        object
            .seek(SeekFrom::Start(3))
            .expect("seek should update position");
        let mut second = String::new();
        object
            .read_to_string(&mut second)
            .expect("range should read after seek");

        assert_eq!(&first, b"ab");
        assert_eq!(second, "def");
        assert_eq!(object.source_ref().client().head_requests().len(), 1);
        assert_eq!(
            object.source_ref().client().range_requests()[0].range_header(),
            "bytes=0-"
        );
        assert_eq!(
            object.source_ref().client().range_requests()[1].range_header(),
            "bytes=3-"
        );
    }

    #[test]
    fn stub_matches_versioned_objects() {
        let mut client = S3ObjectClientStub::new().with_versioned_object(
            "orders",
            "order.json",
            "version-1",
            b"versioned",
        );
        let request = S3HeadObjectRequest::new(
            S3ObjectIdentifier::new("orders", "order.json").with_version_id("version-1"),
        );

        let output = client
            .head_object(request)
            .expect("versioned object should exist");

        assert_eq!(output.content_length(), 9);
        assert_eq!(
            client.head_requests()[0].object().version_id(),
            Some("version-1")
        );
    }

    #[test]
    fn stub_reports_missing_objects() {
        let mut client = S3ObjectClientStub::new();
        let request = S3HeadObjectRequest::new(S3ObjectIdentifier::new("orders", "missing.json"));

        let error = client
            .head_object(request)
            .expect_err("missing object should error");

        assert_eq!(error.kind(), StreamingErrorKind::Io);
        assert!(error.message().contains("bucket orders key missing.json"));
        assert!(client.head_requests().is_empty());
    }
}
