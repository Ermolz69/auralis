use std::io;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

use crate::protocol::MAX_FRAME_BYTES;

pub struct WorkerFrameReader<R> {
    stream: BufReader<R>,
}

impl<R: AsyncRead + Unpin> WorkerFrameReader<R> {
    pub fn new(stream: R) -> Self {
        Self {
            stream: BufReader::new(stream),
        }
    }

    pub async fn next_frame(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut frame = Vec::new();
        loop {
            let available = self.stream.fill_buf().await?;
            if available.is_empty() {
                if frame.is_empty() {
                    return Ok(None);
                }
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TTS worker ended with a partial frame",
                ));
            }
            let count = available
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(available.len(), |index| index + 1);
            if frame.len() + count > MAX_FRAME_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "TTS worker frame exceeds the byte limit",
                ));
            }
            let complete = available[count - 1] == b'\n';
            frame.extend_from_slice(&available[..count]);
            self.stream.consume(count);
            if complete {
                return Ok(Some(frame));
            }
        }
    }
}
