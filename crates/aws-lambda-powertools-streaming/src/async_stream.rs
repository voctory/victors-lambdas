//! Asynchronous seekable range stream.

use std::{
    fmt,
    io::{self, SeekFrom},
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};

use crate::{AsyncRangeFuture, AsyncRangeSource, StreamingError};

/// Asynchronous seekable reader backed by an async range source.
///
/// Seeking invalidates the current reader only when the target position changes. The next read then
/// opens a new range from that byte position.
pub struct AsyncSeekableStream<S>
where
    S: AsyncRangeSource,
{
    source: S,
    position: u64,
    reader: Option<S::Reader>,
    opening_reader: Option<AsyncRangeFuture<S::Reader>>,
    length: Option<u64>,
    pending_seek: Option<PendingSeek>,
}

struct PendingSeek {
    offset: i64,
    length: AsyncRangeFuture<u64>,
}

impl<S> fmt::Debug for AsyncSeekableStream<S>
where
    S: AsyncRangeSource + fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AsyncSeekableStream")
            .field("source", &self.source)
            .field("position", &self.position)
            .field("has_reader", &self.reader.is_some())
            .field("is_opening_reader", &self.opening_reader.is_some())
            .field("length", &self.length)
            .field("has_pending_seek", &self.pending_seek.is_some())
            .finish()
    }
}

impl<S> AsyncSeekableStream<S>
where
    S: AsyncRangeSource,
{
    /// Creates an async seekable stream from an async range source.
    #[must_use]
    pub const fn new(source: S) -> Self {
        Self {
            source,
            position: 0,
            reader: None,
            opening_reader: None,
            length: None,
            pending_seek: None,
        }
    }

    /// Returns a reference to the underlying async range source.
    #[must_use]
    pub const fn source_ref(&self) -> &S {
        &self.source
    }

    /// Returns a mutable reference to the underlying async range source.
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// Returns the total stream length in bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying source cannot report its length.
    pub async fn len(&mut self) -> crate::StreamingResult<u64> {
        if let Some(length) = self.length {
            return Ok(length);
        }

        let length = self.source.len().await?;
        self.length = Some(length);
        Ok(length)
    }

    /// Returns true when the stream is empty.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying source cannot report its length.
    pub async fn is_empty(&mut self) -> crate::StreamingResult<bool> {
        self.len().await.map(|length| length == 0)
    }

    fn set_position(&mut self, position: u64) {
        if self.position != position {
            self.reader = None;
            self.opening_reader = None;
        }
        self.position = position;
    }
}

impl<S> AsyncRead for AsyncSeekableStream<S>
where
    S: AsyncRangeSource + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if buffer.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        let this = self.get_mut();

        loop {
            if let Some(reader) = this.reader.as_mut() {
                let remaining = buffer.remaining();
                let read = Pin::new(reader).poll_read(context, buffer);

                if let Poll::Ready(Ok(())) = &read {
                    let bytes_read = remaining - buffer.remaining();
                    this.position = this
                        .position
                        .checked_add(bytes_read as u64)
                        .ok_or_else(|| io::Error::other("stream position overflow"))?;
                }

                return read;
            }

            if this.opening_reader.is_none() {
                this.opening_reader = Some(this.source.open_range(this.position));
            }

            let open_result = this
                .opening_reader
                .as_mut()
                .expect("opening reader exists")
                .as_mut()
                .poll(context);

            match open_result {
                Poll::Ready(Ok(reader)) => {
                    this.reader = Some(reader);
                    this.opening_reader = None;
                }
                Poll::Ready(Err(error)) => {
                    this.opening_reader = None;
                    return Poll::Ready(Err(error.into_io_error()));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<S> AsyncSeek for AsyncSeekableStream<S>
where
    S: AsyncRangeSource + Unpin,
{
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        let this = self.get_mut();
        if this.pending_seek.is_some() {
            return Err(io::Error::other("stream seek already in progress"));
        }

        match position {
            SeekFrom::Start(offset) => {
                this.set_position(offset);
                Ok(())
            }
            SeekFrom::Current(offset) => {
                let target = seek_target(this.position, offset)?;
                this.set_position(target);
                Ok(())
            }
            SeekFrom::End(offset) => {
                if let Some(length) = this.length {
                    let target = seek_target(length, offset)?;
                    this.set_position(target);
                    return Ok(());
                }

                this.pending_seek = Some(PendingSeek {
                    offset,
                    length: this.source.len(),
                });
                Ok(())
            }
        }
    }

    fn poll_complete(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<u64>> {
        let this = self.get_mut();
        let Some(mut pending_seek) = this.pending_seek.take() else {
            return Poll::Ready(Ok(this.position));
        };

        match pending_seek.length.as_mut().poll(context) {
            Poll::Ready(Ok(length)) => {
                this.length = Some(length);
                let target = seek_target(length, pending_seek.offset)?;
                this.set_position(target);
                Poll::Ready(Ok(this.position))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into_io_error())),
            Poll::Pending => {
                this.pending_seek = Some(pending_seek);
                Poll::Pending
            }
        }
    }
}

fn seek_target(base: u64, offset: i64) -> io::Result<u64> {
    let target = i128::from(base) + i128::from(offset);
    if target < 0 {
        return Err(StreamingError::invalid_seek(target).into_io_error());
    }

    u64::try_from(target).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "stream seek target is too large",
        )
    })
}

#[cfg(test)]
mod tests {
    use std::io::SeekFrom;

    use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};

    use crate::BytesRangeSource;

    use super::*;

    #[test]
    fn async_stream_reads_from_current_position() {
        futures_executor::block_on(async {
            let source = BytesRangeSource::new(b"abcdef".to_vec());
            let mut stream = AsyncSeekableStream::new(source);
            let mut first = [0; 2];
            let mut second = [0; 3];

            stream
                .read_exact(&mut first)
                .await
                .expect("read should succeed");
            stream
                .read_exact(&mut second)
                .await
                .expect("read should succeed");

            assert_eq!(&first, b"ab");
            assert_eq!(&second, b"cde");
            assert_eq!(stream.source_ref().open_count(), 1);
        });
    }

    #[test]
    fn async_stream_reopens_range_after_seek() {
        futures_executor::block_on(async {
            let source = BytesRangeSource::new(b"abcdef".to_vec());
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
            assert_eq!(stream.source_ref().open_count(), 2);
        });
    }

    #[test]
    fn async_stream_supports_seek_from_end() {
        futures_executor::block_on(async {
            let source = BytesRangeSource::new(b"abcdef".to_vec());
            let mut stream = AsyncSeekableStream::new(source);
            let mut buffer = [0; 2];

            stream
                .seek(SeekFrom::End(-2))
                .await
                .expect("seek should succeed");
            stream
                .read_exact(&mut buffer)
                .await
                .expect("read should succeed");

            assert_eq!(&buffer, b"ef");
        });
    }

    #[test]
    fn async_stream_rejects_negative_seek() {
        futures_executor::block_on(async {
            let source = BytesRangeSource::new(b"abcdef".to_vec());
            let mut stream = AsyncSeekableStream::new(source);

            let error = stream
                .seek(SeekFrom::Current(-1))
                .await
                .expect_err("seek should fail");

            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        });
    }
}
