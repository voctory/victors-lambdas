//! Range-backed sources.

use std::{
    io::{Cursor, Read},
    sync::Arc,
};

use crate::StreamingResult;

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

/// In-memory range source for local data, examples, and tests.
#[derive(Clone, Debug, Default)]
pub struct BytesRangeSource {
    data: Arc<Vec<u8>>,
    open_count: usize,
}

impl BytesRangeSource {
    /// Creates an in-memory range source.
    #[must_use]
    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        Self {
            data: Arc::new(data.into()),
            open_count: 0,
        }
    }

    /// Returns the number of ranges opened by this source.
    #[must_use]
    pub const fn open_count(&self) -> usize {
        self.open_count
    }
}

impl RangeSource for BytesRangeSource {
    type Reader = Cursor<Vec<u8>>;

    fn len(&mut self) -> StreamingResult<u64> {
        Ok(self.data.len() as u64)
    }

    fn open_range(&mut self, offset: u64) -> StreamingResult<Self::Reader> {
        self.open_count += 1;
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        let data = self
            .data
            .get(offset..)
            .map_or_else(Vec::new, ToOwned::to_owned);

        Ok(Cursor::new(data))
    }
}
