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
