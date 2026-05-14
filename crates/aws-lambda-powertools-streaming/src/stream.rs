//! Seekable range stream.

use std::io::{self, Read, Seek, SeekFrom};

use crate::{RangeSource, StreamingError, StreamingResult};

/// Seekable reader backed by a range source.
///
/// Seeking invalidates the current reader only when the target position changes. The next read then
/// opens a new range from that byte position.
#[derive(Clone, Debug)]
pub struct SeekableStream<S>
where
    S: RangeSource,
{
    source: S,
    position: u64,
    reader: Option<S::Reader>,
    length: Option<u64>,
}

impl<S> SeekableStream<S>
where
    S: RangeSource,
{
    /// Creates a seekable stream from a range source.
    #[must_use]
    pub const fn new(source: S) -> Self {
        Self {
            source,
            position: 0,
            reader: None,
            length: None,
        }
    }

    /// Returns a reference to the underlying range source.
    #[must_use]
    pub const fn source_ref(&self) -> &S {
        &self.source
    }

    /// Returns a mutable reference to the underlying range source.
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// Returns the total stream length in bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying source cannot report its length.
    pub fn len(&mut self) -> StreamingResult<u64> {
        if let Some(length) = self.length {
            return Ok(length);
        }

        let length = self.source.len()?;
        self.length = Some(length);
        Ok(length)
    }

    /// Returns true when the stream is empty.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying source cannot report its length.
    pub fn is_empty(&mut self) -> StreamingResult<bool> {
        self.len().map(|length| length == 0)
    }

    fn ensure_reader(&mut self) -> io::Result<&mut S::Reader> {
        if self.reader.is_none() {
            let reader = self
                .source
                .open_range(self.position)
                .map_err(StreamingError::into_io_error)?;
            self.reader = Some(reader);
        }

        Ok(self.reader.as_mut().expect("reader exists"))
    }

    fn set_position(&mut self, position: u64) {
        if self.position != position {
            self.reader = None;
        }
        self.position = position;
    }
}

impl<S> Read for SeekableStream<S>
where
    S: RangeSource,
{
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.ensure_reader()?.read(buffer)?;
        self.position = self
            .position
            .checked_add(read as u64)
            .ok_or_else(|| io::Error::other("stream position overflow"))?;
        Ok(read)
    }
}

impl<S> Seek for SeekableStream<S>
where
    S: RangeSource,
{
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let target = match position {
            SeekFrom::Start(offset) => i128::from(offset),
            SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
            SeekFrom::End(offset) => {
                i128::from(self.len().map_err(StreamingError::into_io_error)?) + i128::from(offset)
            }
        };

        if target < 0 {
            return Err(StreamingError::invalid_seek(target).into_io_error());
        }

        let target = u64::try_from(target).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "stream seek target is too large",
            )
        })?;
        self.set_position(target);
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read as _, Seek as _};

    use crate::BytesRangeSource;

    use super::*;

    #[test]
    fn reads_from_current_position() {
        let source = BytesRangeSource::new(b"abcdef".to_vec());
        let mut stream = SeekableStream::new(source);
        let mut first = [0; 2];
        let mut second = [0; 3];

        stream.read_exact(&mut first).expect("read should succeed");
        stream.read_exact(&mut second).expect("read should succeed");

        assert_eq!(&first, b"ab");
        assert_eq!(&second, b"cde");
        assert_eq!(stream.source_ref().open_count(), 1);
    }

    #[test]
    fn reopens_range_after_seek() {
        let source = BytesRangeSource::new(b"abcdef".to_vec());
        let mut stream = SeekableStream::new(source);
        let mut buffer = [0; 2];

        stream.read_exact(&mut buffer).expect("read should succeed");
        stream
            .seek(SeekFrom::Start(3))
            .expect("seek should succeed");
        stream.read_exact(&mut buffer).expect("read should succeed");

        assert_eq!(&buffer, b"de");
        assert_eq!(stream.source_ref().open_count(), 2);
    }

    #[test]
    fn supports_seek_from_end() {
        let source = BytesRangeSource::new(b"abcdef".to_vec());
        let mut stream = SeekableStream::new(source);
        let mut buffer = [0; 2];

        stream.seek(SeekFrom::End(-2)).expect("seek should succeed");
        stream.read_exact(&mut buffer).expect("read should succeed");

        assert_eq!(&buffer, b"ef");
    }

    #[test]
    fn rejects_negative_seek() {
        let source = BytesRangeSource::new(b"abcdef".to_vec());
        let mut stream = SeekableStream::new(source);

        let error = stream
            .seek(SeekFrom::Current(-1))
            .expect_err("seek should fail");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn works_with_buffered_line_reading() {
        let source = BytesRangeSource::new(b"first\nsecond\n".to_vec());
        let stream = SeekableStream::new(source);
        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        reader.read_line(&mut line).expect("line should read");

        assert_eq!(line, "first\n");
    }
}
