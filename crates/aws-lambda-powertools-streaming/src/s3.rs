//! S3 range source abstraction.

use std::io::Read;

use crate::{RangeSource, StreamingResult};

/// Identifies an S3 object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct S3ObjectIdentifier {
    bucket: String,
    key: String,
    version_id: Option<String>,
}

impl S3ObjectIdentifier {
    /// Creates an S3 object identifier.
    #[must_use]
    pub fn new(bucket: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            key: key.into(),
            version_id: None,
        }
    }

    /// Sets the S3 object version ID.
    #[must_use]
    pub fn with_version_id(mut self, version_id: impl Into<String>) -> Self {
        self.version_id = Some(version_id.into());
        self
    }

    /// Returns the S3 bucket name.
    #[must_use]
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// Returns the S3 object key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the S3 object version ID.
    #[must_use]
    pub fn version_id(&self) -> Option<&str> {
        self.version_id.as_deref()
    }
}

/// Request to retrieve S3 object metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct S3HeadObjectRequest {
    object: S3ObjectIdentifier,
}

impl S3HeadObjectRequest {
    /// Creates a head-object request.
    #[must_use]
    pub const fn new(object: S3ObjectIdentifier) -> Self {
        Self { object }
    }

    /// Returns the target S3 object.
    #[must_use]
    pub const fn object(&self) -> &S3ObjectIdentifier {
        &self.object
    }
}

/// Output from an S3 object metadata request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct S3HeadObjectOutput {
    content_length: u64,
}

impl S3HeadObjectOutput {
    /// Creates a head-object output value.
    #[must_use]
    pub const fn new(content_length: u64) -> Self {
        Self { content_length }
    }

    /// Returns the S3 object content length in bytes.
    #[must_use]
    pub const fn content_length(&self) -> u64 {
        self.content_length
    }
}

/// Request to open an S3 object byte range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct S3GetObjectRangeRequest {
    object: S3ObjectIdentifier,
    offset: u64,
    range_header: String,
}

impl S3GetObjectRangeRequest {
    /// Creates a get-object range request.
    #[must_use]
    pub fn new(object: S3ObjectIdentifier, offset: u64) -> Self {
        Self {
            object,
            offset,
            range_header: format!("bytes={offset}-"),
        }
    }

    /// Returns the target S3 object.
    #[must_use]
    pub const fn object(&self) -> &S3ObjectIdentifier {
        &self.object
    }

    /// Returns the byte offset where the range starts.
    #[must_use]
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the HTTP `Range` header value.
    #[must_use]
    pub fn range_header(&self) -> &str {
        &self.range_header
    }
}

/// Client abstraction used by `S3RangeSource`.
pub trait S3ObjectClient {
    /// Reader returned for an opened S3 object range.
    type Reader: Read;

    /// Retrieves object metadata.
    ///
    /// # Errors
    ///
    /// Returns an error when the client cannot retrieve object metadata.
    fn head_object(&mut self, request: S3HeadObjectRequest) -> StreamingResult<S3HeadObjectOutput>;

    /// Opens an object body reader from a byte offset.
    ///
    /// # Errors
    ///
    /// Returns an error when the client cannot open the requested object range.
    fn get_object_range(
        &mut self,
        request: S3GetObjectRangeRequest,
    ) -> StreamingResult<Self::Reader>;
}

/// Range source for an S3 object.
#[derive(Clone, Debug)]
pub struct S3RangeSource<C> {
    object: S3ObjectIdentifier,
    client: C,
    length: Option<u64>,
}

impl<C> S3RangeSource<C>
where
    C: S3ObjectClient,
{
    /// Creates an S3 range source.
    #[must_use]
    pub const fn new(object: S3ObjectIdentifier, client: C) -> Self {
        Self {
            object,
            client,
            length: None,
        }
    }

    /// Creates an S3 range source for a bucket and object key.
    #[must_use]
    pub fn for_bucket_key(bucket: impl Into<String>, key: impl Into<String>, client: C) -> Self {
        Self::new(S3ObjectIdentifier::new(bucket, key), client)
    }

    /// Returns the target S3 object.
    #[must_use]
    pub const fn object(&self) -> &S3ObjectIdentifier {
        &self.object
    }

    /// Returns a reference to the S3 client abstraction.
    #[must_use]
    pub const fn client(&self) -> &C {
        &self.client
    }

    /// Returns a mutable reference to the S3 client abstraction.
    pub fn client_mut(&mut self) -> &mut C {
        &mut self.client
    }

    /// Consumes this range source and returns the S3 client abstraction.
    pub fn into_client(self) -> C {
        self.client
    }
}

impl<C> RangeSource for S3RangeSource<C>
where
    C: S3ObjectClient,
{
    type Reader = C::Reader;

    fn len(&mut self) -> StreamingResult<u64> {
        if let Some(length) = self.length {
            return Ok(length);
        }

        let request = S3HeadObjectRequest::new(self.object.clone());
        let output = self.client.head_object(request)?;
        let length = output.content_length();
        self.length = Some(length);
        Ok(length)
    }

    fn open_range(&mut self, offset: u64) -> StreamingResult<Self::Reader> {
        let request = S3GetObjectRangeRequest::new(self.object.clone(), offset);
        self.client.get_object_range(request)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read as _, Seek as _, SeekFrom};

    use crate::SeekableStream;

    use super::*;

    #[derive(Clone, Debug)]
    struct FakeS3Client {
        data: Vec<u8>,
        head_requests: Vec<S3HeadObjectRequest>,
        range_requests: Vec<S3GetObjectRangeRequest>,
    }

    impl FakeS3Client {
        fn new(data: impl Into<Vec<u8>>) -> Self {
            Self {
                data: data.into(),
                head_requests: Vec::new(),
                range_requests: Vec::new(),
            }
        }
    }

    impl S3ObjectClient for FakeS3Client {
        type Reader = Cursor<Vec<u8>>;

        fn head_object(
            &mut self,
            request: S3HeadObjectRequest,
        ) -> StreamingResult<S3HeadObjectOutput> {
            self.head_requests.push(request);
            Ok(S3HeadObjectOutput::new(self.data.len() as u64))
        }

        fn get_object_range(
            &mut self,
            request: S3GetObjectRangeRequest,
        ) -> StreamingResult<Self::Reader> {
            let offset = usize::try_from(request.offset()).unwrap_or(usize::MAX);
            self.range_requests.push(request);
            let data = self
                .data
                .get(offset..)
                .map_or_else(Vec::new, ToOwned::to_owned);

            Ok(Cursor::new(data))
        }
    }

    #[test]
    fn exposes_s3_object_identity() {
        let object = S3ObjectIdentifier::new("bucket", "key").with_version_id("version-1");

        assert_eq!(object.bucket(), "bucket");
        assert_eq!(object.key(), "key");
        assert_eq!(object.version_id(), Some("version-1"));
    }

    #[test]
    fn creates_source_for_bucket_and_key() {
        let client = FakeS3Client::new(b"abcdef".to_vec());
        let source = S3RangeSource::for_bucket_key("bucket", "key", client);

        assert_eq!(source.object().bucket(), "bucket");
        assert_eq!(source.object().key(), "key");
        assert_eq!(source.object().version_id(), None);
    }

    #[test]
    fn opens_s3_ranges_from_seekable_stream() {
        let object = S3ObjectIdentifier::new("bucket", "key");
        let client = FakeS3Client::new(b"abcdef".to_vec());
        let source = S3RangeSource::new(object, client);
        let mut stream = SeekableStream::new(source);
        let mut buffer = [0; 2];

        stream.read_exact(&mut buffer).expect("read should succeed");
        stream
            .seek(SeekFrom::Start(3))
            .expect("seek should succeed");
        stream.read_exact(&mut buffer).expect("read should succeed");

        let client = stream.source_ref().client();
        let ranges: Vec<_> = client
            .range_requests
            .iter()
            .map(S3GetObjectRangeRequest::range_header)
            .collect();
        assert_eq!(&buffer, b"de");
        assert_eq!(ranges, vec!["bytes=0-", "bytes=3-"]);
    }

    #[test]
    fn caches_s3_object_length() {
        let object = S3ObjectIdentifier::new("bucket", "key");
        let client = FakeS3Client::new(b"abcdef".to_vec());
        let source = S3RangeSource::new(object, client);
        let mut stream = SeekableStream::new(source);

        assert_eq!(stream.len().expect("length should load"), 6);
        assert_eq!(stream.len().expect("length should be cached"), 6);

        assert_eq!(stream.source_ref().client().head_requests.len(), 1);
    }

    #[test]
    fn preserves_s3_object_identity_in_requests() {
        let object = S3ObjectIdentifier::new("bucket", "key").with_version_id("version-1");
        let client = FakeS3Client::new(b"abcdef".to_vec());
        let source = S3RangeSource::new(object, client);
        let mut stream = SeekableStream::new(source);
        let mut buffer = [0; 2];

        assert_eq!(stream.len().expect("length should load"), 6);
        stream
            .seek(SeekFrom::Start(3))
            .expect("seek should succeed");
        stream.read_exact(&mut buffer).expect("read should succeed");

        let client = stream.source_ref().client();
        let head_object = client.head_requests[0].object();
        let range_object = client.range_requests[0].object();
        assert_eq!(head_object.bucket(), "bucket");
        assert_eq!(head_object.key(), "key");
        assert_eq!(head_object.version_id(), Some("version-1"));
        assert_eq!(range_object.bucket(), "bucket");
        assert_eq!(range_object.key(), "key");
        assert_eq!(range_object.version_id(), Some("version-1"));
    }
}
