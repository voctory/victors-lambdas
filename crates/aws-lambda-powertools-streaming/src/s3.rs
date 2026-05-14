//! S3 range source abstraction.

use std::io::{self, Read, Seek, SeekFrom};

#[cfg(feature = "s3")]
use std::{
    future::Future,
    io::Cursor,
    pin::Pin,
    sync::mpsc::{self, Receiver, SyncSender},
};

#[cfg(feature = "s3")]
use aws_sdk_s3::{Client as AwsSdkS3Client, primitives::ByteStream};
#[cfg(feature = "async")]
use tokio::io::AsyncRead;
#[cfg(feature = "s3")]
use tokio::io::{AsyncBufRead, AsyncReadExt as _};

#[cfg(feature = "async")]
use crate::{AsyncRangeFuture, AsyncRangeSource};
use crate::{RangeSource, SeekableStream, StreamingResult};
#[cfg(feature = "s3")]
use crate::{StreamingError, StreamingErrorKind};

#[cfg(feature = "s3")]
const S3_READER_CHUNK_SIZE: usize = 8 * 1024;

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

/// Async client abstraction used by `S3RangeSource`.
#[cfg(feature = "async")]
pub trait AsyncS3ObjectClient {
    /// Async reader returned for an opened S3 object range.
    type Reader: AsyncRead + Send + Unpin + 'static;

    /// Retrieves object metadata asynchronously.
    fn head_object(&self, request: S3HeadObjectRequest) -> AsyncRangeFuture<S3HeadObjectOutput>;

    /// Opens an async object body reader from a byte offset.
    fn get_object_range(&self, request: S3GetObjectRangeRequest) -> AsyncRangeFuture<Self::Reader>;
}

/// AWS SDK-backed client abstraction for S3 range reads.
///
/// This adapter accepts a configured [`aws_sdk_s3::Client`] instead of loading
/// AWS SDK configuration itself. Range readers are backed by a small background
/// Tokio runtime so the public [`Read`] interface can stream chunks without
/// collecting the rest of the object into memory.
#[cfg(feature = "s3")]
#[derive(Clone, Debug)]
pub struct AwsSdkS3ObjectClient {
    client: AwsSdkS3Client,
}

/// Async reader over an AWS SDK S3 object body.
#[cfg(feature = "s3")]
pub type AwsSdkS3AsyncRangeReader = Pin<Box<dyn AsyncBufRead + Send>>;

#[cfg(feature = "s3")]
impl AwsSdkS3ObjectClient {
    /// Creates an AWS SDK-backed S3 object client.
    #[must_use]
    pub fn new(client: AwsSdkS3Client) -> Self {
        Self { client }
    }

    /// Returns the underlying AWS SDK S3 client.
    #[must_use]
    pub const fn client(&self) -> &AwsSdkS3Client {
        &self.client
    }
}

#[cfg(feature = "s3")]
impl S3ObjectClient for AwsSdkS3ObjectClient {
    type Reader = AwsSdkS3RangeReader;

    fn head_object(&mut self, request: S3HeadObjectRequest) -> StreamingResult<S3HeadObjectOutput> {
        let client = self.client.clone();
        let object = request.object().clone();

        run_s3_request(head_s3_object(client, object))
    }

    fn get_object_range(
        &mut self,
        request: S3GetObjectRangeRequest,
    ) -> StreamingResult<Self::Reader> {
        open_s3_range_reader(
            self.client.clone(),
            request.object().clone(),
            request.range_header().to_owned(),
        )
    }
}

#[cfg(feature = "s3")]
impl AsyncS3ObjectClient for AwsSdkS3ObjectClient {
    type Reader = AwsSdkS3AsyncRangeReader;

    fn head_object(&self, request: S3HeadObjectRequest) -> AsyncRangeFuture<S3HeadObjectOutput> {
        let client = self.client.clone();
        let object = request.object().clone();

        Box::pin(head_s3_object(client, object))
    }

    fn get_object_range(&self, request: S3GetObjectRangeRequest) -> AsyncRangeFuture<Self::Reader> {
        let client = self.client.clone();
        let object = request.object().clone();
        let range_header = request.range_header().to_owned();

        Box::pin(async move {
            let mut request = client
                .get_object()
                .bucket(object.bucket().to_owned())
                .key(object.key().to_owned())
                .range(range_header);
            if let Some(version_id) = object.version_id() {
                request = request.version_id(version_id.to_owned());
            }

            let output = request
                .send()
                .await
                .map_err(|error| sdk_error("get_object", error))?;
            let reader: AwsSdkS3AsyncRangeReader = Box::pin(output.body.into_async_read());
            Ok(reader)
        })
    }
}

#[cfg(feature = "s3")]
async fn head_s3_object(
    client: AwsSdkS3Client,
    object: S3ObjectIdentifier,
) -> StreamingResult<S3HeadObjectOutput> {
    let mut request = client
        .head_object()
        .bucket(object.bucket().to_owned())
        .key(object.key().to_owned());
    if let Some(version_id) = object.version_id() {
        request = request.version_id(version_id.to_owned());
    }

    let output = request
        .send()
        .await
        .map_err(|error| sdk_error("head_object", error))?;
    let content_length = output.content_length().unwrap_or_default();
    let content_length = u64::try_from(content_length).map_err(|_| {
        StreamingError::new(
            StreamingErrorKind::Io,
            "S3 head_object returned a negative content length",
        )
    })?;

    Ok(S3HeadObjectOutput::new(content_length))
}

/// Synchronous reader over an AWS SDK S3 object body.
#[cfg(feature = "s3")]
pub struct AwsSdkS3RangeReader {
    receiver: Receiver<io::Result<Vec<u8>>>,
    current: Cursor<Vec<u8>>,
    done: bool,
}

#[cfg(feature = "s3")]
impl AwsSdkS3RangeReader {
    fn new(receiver: Receiver<io::Result<Vec<u8>>>) -> Self {
        Self {
            receiver,
            current: Cursor::new(Vec::new()),
            done: false,
        }
    }

    #[cfg(test)]
    fn from_body(body: ByteStream) -> StreamingResult<Self> {
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("powertools-s3-range-reader".to_owned())
            .spawn(move || run_s3_body_worker(body, sender))
            .map_err(StreamingError::io)?;

        Ok(Self::new(receiver))
    }
}

#[cfg(feature = "s3")]
impl Read for AwsSdkS3RangeReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            let read = Read::read(&mut self.current, buffer)?;
            if read > 0 {
                return Ok(read);
            }

            if self.done {
                return Ok(0);
            }

            match self.receiver.recv() {
                Ok(Ok(chunk)) => {
                    self.current = Cursor::new(chunk);
                }
                Ok(Err(error)) => {
                    self.done = true;
                    return Err(error);
                }
                Err(_) => {
                    self.done = true;
                    return Ok(0);
                }
            }
        }
    }
}

#[cfg(feature = "s3")]
fn open_s3_range_reader(
    client: AwsSdkS3Client,
    object: S3ObjectIdentifier,
    range_header: String,
) -> StreamingResult<AwsSdkS3RangeReader> {
    let (init_sender, init_receiver) = mpsc::sync_channel(1);
    let (chunk_sender, chunk_receiver) = mpsc::sync_channel(1);

    std::thread::Builder::new()
        .name("powertools-s3-range-reader".to_owned())
        .spawn(move || {
            let runtime = match s3_runtime() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = init_sender.send(Err(error));
                    return;
                }
            };

            runtime.block_on(async move {
                let mut request = client
                    .get_object()
                    .bucket(object.bucket().to_owned())
                    .key(object.key().to_owned())
                    .range(range_header);
                if let Some(version_id) = object.version_id() {
                    request = request.version_id(version_id.to_owned());
                }

                let output = match request.send().await {
                    Ok(output) => output,
                    Err(error) => {
                        let _ = init_sender.send(Err(sdk_error("get_object", error)));
                        return;
                    }
                };

                if init_sender.send(Ok(())).is_err() {
                    return;
                }

                stream_s3_body(output.body, chunk_sender).await;
            });
        })
        .map_err(StreamingError::io)?;

    match init_receiver.recv() {
        Ok(Ok(())) => Ok(AwsSdkS3RangeReader::new(chunk_receiver)),
        Ok(Err(error)) => Err(error),
        Err(error) => Err(StreamingError::io(io::Error::other(error))),
    }
}

#[cfg(feature = "s3")]
fn run_s3_request<F, T>(future: F) -> StreamingResult<T>
where
    F: Future<Output = StreamingResult<T>> + Send + 'static,
    T: Send + 'static,
{
    let worker = std::thread::Builder::new()
        .name("powertools-s3-request".to_owned())
        .spawn(move || {
            let runtime = s3_runtime()?;
            runtime.block_on(future)
        })
        .map_err(StreamingError::io)?;

    worker.join().map_err(|_| {
        StreamingError::new(StreamingErrorKind::Io, "S3 request worker thread panicked")
    })?
}

#[cfg(all(feature = "s3", test))]
fn run_s3_body_worker(body: ByteStream, sender: SyncSender<io::Result<Vec<u8>>>) {
    match s3_runtime() {
        Ok(runtime) => runtime.block_on(stream_s3_body(body, sender)),
        Err(error) => {
            let _ = sender.send(Err(error.into_io_error()));
        }
    }
}

#[cfg(feature = "s3")]
async fn stream_s3_body(body: ByteStream, sender: SyncSender<io::Result<Vec<u8>>>) {
    let mut reader = body.into_async_read();
    let mut buffer = vec![0; S3_READER_CHUNK_SIZE];

    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => break,
            Ok(read) => {
                if sender.send(Ok(buffer[..read].to_vec())).is_err() {
                    break;
                }
            }
            Err(error) => {
                let _ = sender.send(Err(error));
                break;
            }
        }
    }
}

#[cfg(feature = "s3")]
fn s3_runtime() -> StreamingResult<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(StreamingError::io)
}

#[cfg(feature = "s3")]
fn sdk_error(operation: &str, error: impl std::fmt::Display) -> StreamingError {
    StreamingError::new(
        StreamingErrorKind::Io,
        format!("S3 {operation} request failed: {error}"),
    )
}

/// Range source for an S3 object.
#[derive(Clone, Debug)]
pub struct S3RangeSource<C> {
    object: S3ObjectIdentifier,
    client: C,
    length: Option<u64>,
}

impl<C> S3RangeSource<C> {
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

/// Seekable reader for an S3 object.
///
/// This is a convenience wrapper around [`SeekableStream`] and [`S3RangeSource`] for callers who
/// want a higher-level S3 object entry point while still supplying an explicit S3 client
/// abstraction.
pub struct S3Object<C>
where
    C: S3ObjectClient,
{
    stream: SeekableStream<S3RangeSource<C>>,
}

impl<C> S3Object<C>
where
    C: S3ObjectClient,
{
    /// Creates a seekable S3 object reader.
    #[must_use]
    pub fn new(object: S3ObjectIdentifier, client: C) -> Self {
        Self {
            stream: SeekableStream::new(S3RangeSource::new(object, client)),
        }
    }

    /// Creates a seekable S3 object reader for a bucket and object key.
    #[must_use]
    pub fn for_bucket_key(bucket: impl Into<String>, key: impl Into<String>, client: C) -> Self {
        Self::new(S3ObjectIdentifier::new(bucket, key), client)
    }

    /// Creates a seekable S3 object reader for a versioned object.
    #[must_use]
    pub fn for_bucket_key_version(
        bucket: impl Into<String>,
        key: impl Into<String>,
        version_id: impl Into<String>,
        client: C,
    ) -> Self {
        Self::new(
            S3ObjectIdentifier::new(bucket, key).with_version_id(version_id),
            client,
        )
    }

    /// Returns the target S3 object.
    #[must_use]
    pub fn object(&self) -> &S3ObjectIdentifier {
        self.stream.source_ref().object()
    }

    /// Returns a reference to the underlying range source.
    #[must_use]
    pub const fn source_ref(&self) -> &S3RangeSource<C> {
        self.stream.source_ref()
    }

    /// Returns a mutable reference to the underlying range source.
    pub fn source_mut(&mut self) -> &mut S3RangeSource<C> {
        self.stream.source_mut()
    }

    /// Consumes this object and returns the underlying seekable stream.
    #[must_use]
    pub fn into_inner(self) -> SeekableStream<S3RangeSource<C>> {
        self.stream
    }

    /// Returns the S3 object size in bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying client cannot retrieve object metadata.
    pub fn size(&mut self) -> StreamingResult<u64> {
        self.stream.len()
    }
}

impl<C> Read for S3Object<C>
where
    C: S3ObjectClient,
{
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buffer)
    }
}

impl<C> Seek for S3Object<C>
where
    C: S3ObjectClient,
{
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.stream.seek(position)
    }
}

#[cfg(feature = "async")]
impl<C> AsyncRangeSource for S3RangeSource<C>
where
    C: AsyncS3ObjectClient,
{
    type Reader = C::Reader;

    fn len(&self) -> AsyncRangeFuture<u64> {
        if let Some(length) = self.length {
            return Box::pin(async move { Ok(length) });
        }

        let request = S3HeadObjectRequest::new(self.object.clone());
        let head_object = self.client.head_object(request);
        Box::pin(async move { head_object.await.map(|output| output.content_length()) })
    }

    fn open_range(&self, offset: u64) -> AsyncRangeFuture<Self::Reader> {
        let request = S3GetObjectRangeRequest::new(self.object.clone(), offset);
        self.client.get_object_range(request)
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
    #[cfg(feature = "async")]
    use std::sync::{Arc, Mutex};

    use crate::SeekableStream;
    #[cfg(feature = "async")]
    use crate::{AsyncRangeFuture, AsyncSeekableStream};

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

    #[cfg(feature = "async")]
    #[derive(Clone, Debug)]
    struct FakeAsyncS3Client {
        data: Arc<Vec<u8>>,
        head_requests: Arc<Mutex<Vec<S3HeadObjectRequest>>>,
        range_requests: Arc<Mutex<Vec<S3GetObjectRangeRequest>>>,
    }

    #[cfg(feature = "async")]
    impl FakeAsyncS3Client {
        fn new(data: impl Into<Vec<u8>>) -> Self {
            Self {
                data: Arc::new(data.into()),
                head_requests: Arc::new(Mutex::new(Vec::new())),
                range_requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn range_headers(&self) -> Vec<String> {
            self.range_requests
                .lock()
                .expect("range requests should lock")
                .iter()
                .map(|request| request.range_header().to_owned())
                .collect()
        }
    }

    #[cfg(feature = "async")]
    impl AsyncS3ObjectClient for FakeAsyncS3Client {
        type Reader = Cursor<Vec<u8>>;

        fn head_object(
            &self,
            request: S3HeadObjectRequest,
        ) -> AsyncRangeFuture<S3HeadObjectOutput> {
            let data = Arc::clone(&self.data);
            let head_requests = Arc::clone(&self.head_requests);

            Box::pin(async move {
                head_requests
                    .lock()
                    .expect("head requests should lock")
                    .push(request);
                Ok(S3HeadObjectOutput::new(data.len() as u64))
            })
        }

        fn get_object_range(
            &self,
            request: S3GetObjectRangeRequest,
        ) -> AsyncRangeFuture<Self::Reader> {
            let data = Arc::clone(&self.data);
            let range_requests = Arc::clone(&self.range_requests);

            Box::pin(async move {
                let offset = usize::try_from(request.offset()).unwrap_or(usize::MAX);
                range_requests
                    .lock()
                    .expect("range requests should lock")
                    .push(request);
                let data = data.get(offset..).map_or_else(Vec::new, ToOwned::to_owned);

                Ok(Cursor::new(data))
            })
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
    fn s3_object_reads_ranges_and_reports_size() {
        let client = FakeS3Client::new(b"abcdef".to_vec());
        let mut object = S3Object::for_bucket_key_version("bucket", "key", "version-1", client);
        let mut buffer = [0; 2];

        assert_eq!(object.object().bucket(), "bucket");
        assert_eq!(object.object().key(), "key");
        assert_eq!(object.object().version_id(), Some("version-1"));
        assert_eq!(object.size().expect("size should load"), 6);

        object.read_exact(&mut buffer).expect("read should succeed");
        object
            .seek(SeekFrom::Start(3))
            .expect("seek should succeed");
        object.read_exact(&mut buffer).expect("read should succeed");

        let client = object.source_ref().client();
        let ranges: Vec<_> = client
            .range_requests
            .iter()
            .map(S3GetObjectRangeRequest::range_header)
            .collect();
        assert_eq!(&buffer, b"de");
        assert_eq!(ranges, vec!["bytes=0-", "bytes=3-"]);
        assert_eq!(client.head_requests.len(), 1);
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

    #[cfg(feature = "async")]
    #[test]
    fn opens_async_s3_ranges_from_async_seekable_stream() {
        futures_executor::block_on(async {
            use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};

            let object = S3ObjectIdentifier::new("bucket", "key");
            let client = FakeAsyncS3Client::new(b"abcdef".to_vec());
            let source = S3RangeSource::new(object, client);
            let mut stream = AsyncSeekableStream::new(source);
            let mut buffer = [0; 2];

            stream
                .read_exact(&mut buffer)
                .await
                .expect("read should succeed");
            stream
                .seek(SeekFrom::Start(3))
                .await
                .expect("seek should succeed");
            stream
                .read_exact(&mut buffer)
                .await
                .expect("read should succeed");

            assert_eq!(&buffer, b"de");
            assert_eq!(
                stream.source_ref().client().range_headers(),
                vec!["bytes=0-", "bytes=3-"]
            );
        });
    }

    #[cfg(feature = "async")]
    #[test]
    fn caches_async_s3_object_length_in_stream() {
        futures_executor::block_on(async {
            let object = S3ObjectIdentifier::new("bucket", "key");
            let client = FakeAsyncS3Client::new(b"abcdef".to_vec());
            let source = S3RangeSource::new(object, client);
            let mut stream = AsyncSeekableStream::new(source);

            assert_eq!(stream.len().await.expect("length should load"), 6);
            assert_eq!(stream.len().await.expect("length should be cached"), 6);

            let head_count = stream
                .source_ref()
                .client()
                .head_requests
                .lock()
                .expect("head requests should lock")
                .len();
            assert_eq!(head_count, 1);
        });
    }

    #[cfg(feature = "s3")]
    #[test]
    fn aws_sdk_client_keeps_configured_client() {
        let client = aws_sdk_s3_client();
        let adapter = AwsSdkS3ObjectClient::new(client);

        assert!(adapter.client().config().region().is_some());
    }

    #[cfg(feature = "s3")]
    #[test]
    fn aws_sdk_range_reader_streams_byte_stream() {
        let body = ByteStream::from_static(b"abcdef");
        let mut reader =
            AwsSdkS3RangeReader::from_body(body).expect("range reader should be created");
        let mut output = String::new();

        reader
            .read_to_string(&mut output)
            .expect("range body should read");

        assert_eq!(output, "abcdef");
    }

    #[cfg(feature = "s3")]
    fn aws_sdk_s3_client() -> AwsSdkS3Client {
        use aws_sdk_s3::{
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

        AwsSdkS3Client::from_conf(config)
    }
}
