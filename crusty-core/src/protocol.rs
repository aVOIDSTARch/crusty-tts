//! Plugin protocol v0.1: frame format (4-byte length + payload), handshake JSON.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

pub const PROTOCOL_VERSION: &str = "0.1";

/// First frame sent to plugin: config + selected input/output types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    pub protocol: String,
    pub input_type: String,
    pub output_type: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

impl Handshake {
    pub fn new(input_type: impl Into<String>, output_type: impl Into<String>, config: serde_json::Value) -> Self {
        Self {
            protocol: PROTOCOL_VERSION.to_string(),
            input_type: input_type.into(),
            output_type: output_type.into(),
            config,
        }
    }
}

/// Frame format: 4-byte length (little-endian) + payload. No per-frame content-type.
pub fn write_frame(mut w: impl Write, payload: &[u8]) -> std::io::Result<()> {
    let len = payload.len() as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(payload)?;
    w.flush()?;
    Ok(())
}

/// Read one frame: 4-byte length then payload.
pub fn read_frame(mut r: impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    if r.read_exact(&mut len_buf).is_err() {
        return Ok(None);
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 {
        return Ok(Some(Vec::new()));
    }
    let mut payload = vec![0u8; len];
    r.read_exact(&mut payload)?;
    Ok(Some(payload))
}

/// Structured error frame a plugin may emit before exiting non-zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorFrame {
    #[serde(rename = "type")]
    pub typ: String,
    pub message: String,
    #[serde(default)]
    pub fatal: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn write_read_frame_roundtrip() {
        let payload = b"hello world";
        let mut buf = Vec::new();
        write_frame(&mut buf, payload).unwrap();
        let mut cur = Cursor::new(buf);
        let read = read_frame(&mut cur).unwrap().unwrap();
        assert_eq!(read.as_slice(), payload);
    }

    #[test]
    fn read_frame_empty_len() {
        let mut cur = Cursor::new([0u8; 4]); // length 0
        let read = read_frame(&mut cur).unwrap().unwrap();
        assert!(read.is_empty());
    }

    #[test]
    fn read_frame_eof_returns_none() {
        let mut cur = Cursor::new(&[][..]);
        let read = read_frame(&mut cur).unwrap();
        assert!(read.is_none());
    }

    #[test]
    fn handshake_new() {
        let h = Handshake::new("text/plain", "audio/wav", serde_json::json!({"voice": "en"}));
        assert_eq!(h.protocol, PROTOCOL_VERSION);
        assert_eq!(h.input_type, "text/plain");
        assert_eq!(h.output_type, "audio/wav");
        assert_eq!(h.config.get("voice").and_then(|v| v.as_str()), Some("en"));
    }

    #[test]
    fn error_frame_serialize() {
        let e = ErrorFrame { typ: "error".into(), message: "fail".into(), fatal: true };
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("fail"));
        let e2: ErrorFrame = serde_json::from_str(&j).unwrap();
        assert!(e2.fatal);
    }
}
