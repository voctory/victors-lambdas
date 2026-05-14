//! Range-backed sources.

#[cfg(feature = "async")]
use std::{future::Future, pin::Pin};
use std::{
    io::{Cursor, Read},
    sync::Arc,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::StreamingResult;

#[cfg(feature = "async")]
use tokio::io::AsyncRead;

/// Source that can open a readable stream from a byte offset.
pub trait RangeSource {
    /// Reader returned for an opened byte range.
    type Reader: Read;

    /// Returns the total length of the source in bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying source cannot report its length.
    fn len(&mut self) -> StreamingResult<u64>;

    /// Returns true when the source is empty.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying source cannot report its length.
    fn is_empty(&mut self) -> StreamingResult<bool> {
        self.len().map(|length| length == 0)
    }

    /// Opens a reader starting at `offset`.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying source cannot open the requested range.
    fn open_range(&mut self, offset: u64) -> StreamingResult<Self::Reader>;
}

/// Boxed future returned by asynchronous range sources.
#[cfg(feature = "async")]
pub type AsyncRangeFuture<T> = Pin<Box<dyn Future<Output = StreamingResult<T>> + Send + 'static>>;

/// Source that can asynchronously open a readable stream from a byte offset.
#[cfg(feature = "async")]
pub trait AsyncRangeSource {
    /// Reader returned for an opened byte range.
    type Reader: AsyncRead + Send + Unpin + 'static;

    /// Returns the total length of the source in bytes.
    fn len(&self) -> AsyncRangeFuture<u64>;

    /// Returns true when the source is empty.
    fn is_empty(&self) -> AsyncRangeFuture<bool> {
        let length = self.len();
        Box::pin(async move { length.await.map(|length| length == 0) })
    }

    /// Opens an async reader starting at `offset`.
    fn open_range(&self, offset: u64) -> AsyncRangeFuture<Self::Reader>;
}

/// In-memory range source for local data, examples, and tests.
#[derive(Clone, Debug)]
pub struct BytesRangeSource {
    state: Arc<BytesRangeSourceState>,
}

#[derive(Debug)]
struct BytesRangeSourceState {
    data: Vec<u8>,
    open_count: AtomicUsize,
}

impl BytesRangeSource {
    /// Creates an in-memory range source.
    #[must_use]
    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        Self {
            state: Arc::new(BytesRangeSourceState {
                data: data.into(),
                open_count: AtomicUsize::new(0),
            }),
        }
    }

    /// Returns the number of ranges opened by this source.
    #[must_use]
    pub fn open_count(&self) -> usize {
        self.state.open_count.load(Ordering::Relaxed)
    }
}

impl Default for BytesRangeSource {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl RangeSource for BytesRangeSource {
    type Reader = Cursor<Vec<u8>>;

    fn len(&mut self) -> StreamingResult<u64> {
        Ok(self.state.data.len() as u64)
    }

    fn open_range(&mut self, offset: u64) -> StreamingResult<Self::Reader> {
        self.state.open_count.fetch_add(1, Ordering::Relaxed);
        let data = bytes_from_offset(&self.state.data, offset);

        Ok(Cursor::new(data))
    }
}

#[cfg(feature = "async")]
impl AsyncRangeSource for BytesRangeSource {
    type Reader = Cursor<Vec<u8>>;

    fn len(&self) -> AsyncRangeFuture<u64> {
        let length = self.state.data.len() as u64;
        Box::pin(async move { Ok(length) })
    }

    fn open_range(&self, offset: u64) -> AsyncRangeFuture<Self::Reader> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            state.open_count.fetch_add(1, Ordering::Relaxed);
            Ok(Cursor::new(bytes_from_offset(&state.data, offset)))
        })
    }
}

fn bytes_from_offset(data: &[u8], offset: u64) -> Vec<u8> {
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    data.get(offset..).map_or_else(Vec::new, ToOwned::to_owned)
}
