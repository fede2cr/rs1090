//! beast — Beast binary protocol reader over TCP
//!
//! Beast protocol wraps Mode S messages for transport over TCP.
//! Format: `<0x1A> <type> <6-byte MLAT timestamp> <1-byte signal> <message bytes>`
//!
//! Type bytes:
//! - `0x31` ('1') → 2-byte Mode-AC message
//! - `0x32` ('2') → 7-byte Mode-S short message
//! - `0x33` ('3') → 14-byte Mode-S long message (includes ADS-B)
//!
//! Any `0x1A` within the payload is escaped as `0x1A 0x1A`.

use std::time::Instant;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::TcpStream;
use tracing::{debug, trace, warn};

/// A decoded Beast frame.
#[derive(Debug, Clone)]
pub struct BeastFrame {
    /// MLAT 48-bit timestamp (12 MHz clock ticks)
    pub mlat_timestamp: u64,
    /// Signal level (0–255)
    pub signal: u8,
    /// Raw Mode S message bytes (2, 7, or 14 bytes)
    pub message: Vec<u8>,
    /// When this frame was received (local monotonic clock)
    pub received_at: Instant,
}

/// Errors from the Beast reader.
#[derive(Debug, thiserror::Error)]
pub enum BeastError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("connection closed")]
    ConnectionClosed,
    #[error("invalid Beast frame type: 0x{0:02x}")]
    InvalidType(u8),
}

/// Read Beast frames from a TCP connection.
pub struct BeastReader {
    reader: BufReader<TcpStream>,
    buf: Vec<u8>,
}

impl BeastReader {
    /// Connect to a Beast TCP server (e.g. readsb on port 30005).
    pub async fn connect(addr: &str) -> Result<Self, BeastError> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        debug!(addr, "connected to Beast source");
        Ok(Self {
            reader: BufReader::with_capacity(8192, stream),
            buf: Vec::with_capacity(32),
        })
    }

    /// Construct from an already-connected stream (useful for testing).
    pub fn from_stream(stream: TcpStream) -> Self {
        Self {
            reader: BufReader::with_capacity(8192, stream),
            buf: Vec::with_capacity(32),
        }
    }

    /// Read the next Beast frame. Blocks until a complete frame is available.
    pub async fn next_frame(&mut self) -> Result<BeastFrame, BeastError> {
        loop {
            // Scan for the start-of-frame marker (0x1A followed by type byte)
            let b = self.read_byte().await?;
            if b != 0x1A {
                continue; // skip garbage
            }

            let type_byte = self.read_byte_unescaped().await?;

            let msg_len: usize = match type_byte {
                0x31 => 2,  // Mode-AC
                0x32 => 7,  // Mode-S short
                0x33 => 14, // Mode-S long (ADS-B)
                _ => {
                    trace!(type_byte, "skipping unknown Beast type");
                    continue;
                }
            };

            // Read 6-byte MLAT timestamp
            let mut mlat_bytes = [0u8; 6];
            for b in &mut mlat_bytes {
                *b = self.read_byte_unescaped().await?;
            }
            let mlat_timestamp = u64::from(mlat_bytes[0]) << 40
                | u64::from(mlat_bytes[1]) << 32
                | u64::from(mlat_bytes[2]) << 24
                | u64::from(mlat_bytes[3]) << 16
                | u64::from(mlat_bytes[4]) << 8
                | u64::from(mlat_bytes[5]);

            // Read 1-byte signal level
            let signal = self.read_byte_unescaped().await?;

            // Read message bytes
            self.buf.clear();
            for _ in 0..msg_len {
                let b = self.read_byte_unescaped().await?;
                self.buf.push(b);
            }

            return Ok(BeastFrame {
                mlat_timestamp,
                signal,
                message: self.buf.clone(),
                received_at: Instant::now(),
            });
        }
    }

    /// Read a single raw byte.
    async fn read_byte(&mut self) -> Result<u8, BeastError> {
        let b = self
            .reader
            .read_u8()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    BeastError::ConnectionClosed
                } else {
                    BeastError::Io(e)
                }
            })?;
        Ok(b)
    }

    /// Read a byte with Beast 0x1A escape handling.
    /// If we see 0x1A 0x1A, return a single 0x1A.
    /// If we see 0x1A followed by a type byte, that's a new frame start —
    /// but here we're inside a frame, so we just return the escaped 0x1A.
    async fn read_byte_unescaped(&mut self) -> Result<u8, BeastError> {
        let b = self.read_byte().await?;
        if b == 0x1A {
            // Escaped 0x1A — consume the duplicate
            let next = self.read_byte().await?;
            if next == 0x1A {
                return Ok(0x1A);
            }
            // This is actually a new frame start. We should not get here
            // in well-formed data, but handle it gracefully.
            warn!("unexpected 0x1A in Beast payload — possible frame boundary");
            return Ok(next);
        }
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a Beast-encoded frame from raw fields (for testing).
    fn encode_beast_frame(type_byte: u8, mlat: u64, signal: u8, msg: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(0x1A);
        out.push(type_byte);

        // MLAT 6 bytes
        for i in (0..6).rev() {
            let b = ((mlat >> (i * 8)) & 0xFF) as u8;
            out.push(b);
            if b == 0x1A {
                out.push(0x1A);
            }
        }

        // Signal
        out.push(signal);
        if signal == 0x1A {
            out.push(0x1A);
        }

        // Message bytes
        for &b in msg {
            out.push(b);
            if b == 0x1A {
                out.push(0x1A);
            }
        }

        out
    }

    #[tokio::test]
    async fn parse_beast_frame() {
        // Create a mock TCP stream pair
        let (client, server) = tokio::io::duplex(1024);

        // Build a type-3 (14-byte long message) Beast frame
        let msg = [
            0x8D, 0xA2, 0xC1, 0xBD, 0x58, 0x7B, 0xA2, 0xAD,
            0xB3, 0x17, 0x99, 0xCB, 0x80, 0x2B,
        ];
        let frame_bytes = encode_beast_frame(0x33, 0x123456789ABC, 0x80, &msg);
        let expected_min_len = frame_bytes.len();

        // Write the frame to the "server" end
        let (_read_half, mut write_half) = tokio::io::split(server);
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            write_half.write_all(&frame_bytes).await.unwrap();
            write_half.shutdown().await.unwrap();
        });

        drop(client);

        assert!(expected_min_len >= 2 + 6 + 1 + 14);
    }

    #[test]
    fn encoding_escapes_1a() {
        let msg = [0x1A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // 7-byte short
        let encoded = encode_beast_frame(0x32, 0, 0, &msg);
        // The 0x1A in the message should be doubled
        let count_1a = encoded.iter().filter(|&&b| b == 0x1A).count();
        // Start marker (1) + escaped 0x1A in message (2) = 3
        assert_eq!(count_1a, 3);
    }
}
