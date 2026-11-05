use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub struct Fragmented<'a> {
    bytes: &'a [u8],
    chunks: Vec<usize>,
    position: usize,
    chunk_index: usize,
    remaining: usize,
    reads: usize,
}

impl<'a> Fragmented<'a> {
    pub fn new(bytes: &'a [u8], chunks: Vec<usize>) -> Self {
        assert!(bytes.len() <= super::MAX_INPUT);
        assert!(!chunks.is_empty() && chunks.iter().all(|&n| n > 0));
        Self {
            bytes,
            chunks,
            position: 0,
            chunk_index: 0,
            remaining: 0,
            reads: 0,
        }
    }
}

impl AsyncRead for Fragmented<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // INVARIANT: Every run has finite input, immediate EOF and a read budget;
        // a non-yielding EOF retry loop becomes an actionable fuzz failure.
        self.reads += 1;
        assert!(self.reads <= 100_000, "parser read budget exhausted");
        if self.remaining == 0 {
            self.remaining = self.chunks[self.chunk_index % self.chunks.len()];
            self.chunk_index += 1;
        }
        let n = self
            .remaining
            .min(buf.remaining())
            .min(self.bytes.len() - self.position);
        buf.put_slice(&self.bytes[self.position..self.position + n]);
        self.position += n;
        self.remaining -= n;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for Fragmented<'_> {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
