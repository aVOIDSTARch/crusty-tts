//! Subprocess plugin runner: handshake + framed I/O, and simple stdin/stdout fallback.

use crate::plugin::{PluginOptions, PluginType};
use crate::protocol::{read_frame, write_frame, Handshake};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Run a plugin subprocess: send handshake then payload frames on stdin, read frames from stdout.
/// `executable` is the plugin binary/script path; `input_bytes` is the first (and for v1 often only) payload.
pub fn run_subprocess_plugin_framed(
    executable: &str,
    handshake: &Handshake,
    input_bytes: &[u8],
    options: &PluginOptions,
) -> anyhow::Result<Vec<u8>> {
    let mut cmd = Command::new(executable);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .env("PLUGIN_INPUT", std::str::from_utf8(input_bytes).unwrap_or(""));
    for (k, v) in options {
        cmd.env(format!("PLUGIN_OPT_{}", k.to_uppercase().replace('-', "_")), v);
    }
    let mut child = cmd.spawn()?;

    let mut stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("no stdin"))?;
    let handshake_json = serde_json::to_string(handshake)?;
    write_frame(&mut stdin, handshake_json.as_bytes())?;
    if !input_bytes.is_empty() {
        write_frame(&mut stdin, input_bytes)?;
    }
    drop(stdin);

    let mut stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
    let mut out = Vec::new();
    while let Some(chunk) = read_frame(&mut stdout)? {
        if chunk.is_empty() {
            break;
        }
        out.extend_from_slice(&chunk);
    }
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("plugin exited with {}", status);
    }
    Ok(out)
}

/// Simple runner: write raw input to stdin, read full stdout. No framing.
/// Used when plugin uses PLUGIN_INPUT env and writes raw output to stdout.
pub fn run_subprocess_plugin(
    executable: &str,
    input_bytes: &[u8],
    options: &PluginOptions,
) -> anyhow::Result<Vec<u8>> {
    let mut cmd = Command::new(executable);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .env("PLUGIN_INPUT", std::str::from_utf8(input_bytes).unwrap_or(""));
    for (k, v) in options {
        cmd.env(format!("PLUGIN_OPT_{}", k.to_uppercase().replace('-', "_")), v);
    }
    let mut child = cmd.spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input_bytes)?;
        stdin.flush()?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("plugin exited with {}", output.status);
    }
    Ok(output.stdout)
}

/// Verify a plugin by running a small sample; returns true if output is valid for the type.
pub fn verify_plugin(plugin_path: &str, plugin_type: PluginType) -> bool {
    if !Path::new(plugin_path).exists() {
        eprintln!("Plugin not found: {}", plugin_path);
        return false;
    }
    let sample_input: &[u8] = match plugin_type {
        PluginType::Pre => b"Test input for pre-processor",
        PluginType::Tts => b"Hello, world!",
        PluginType::Post | PluginType::Converter => b"FAKE_AUDIO_BYTES",
    };
    let options = PluginOptions::new();
    let output = match run_subprocess_plugin(plugin_path, sample_input, &options) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Plugin verification failed {}: {}", plugin_path, e);
            return false;
        }
    };
    let valid = match plugin_type {
        PluginType::Pre => std::str::from_utf8(&output).is_ok(),
        PluginType::Tts | PluginType::Converter | PluginType::Post => !output.is_empty(),
    };
    if valid {
        eprintln!("Plugin verification passed: {}", plugin_path);
    } else {
        eprintln!("Plugin verification failed (invalid output): {}", plugin_path);
    }
    valid
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    /// Run subprocess_plugin with a script that echoes stdin to stdout (Unix).
    #[test]
    #[cfg(unix)]
    fn run_subprocess_plugin_echo() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("run.sh");
        fs::write(&script, "#!/bin/sh\ncat\n").unwrap();
        fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let out = run_subprocess_plugin(script.to_str().unwrap(), b"hello", &PluginOptions::new()).unwrap();
        assert_eq!(out, b"hello");
    }

    #[test]
    #[cfg(unix)]
    fn run_subprocess_plugin_env_input() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("run.sh");
        // Script that outputs PLUGIN_INPUT to stdout
        fs::write(&script, "#!/bin/sh\nprintf '%s' \"$PLUGIN_INPUT\"\n").unwrap();
        fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let out = run_subprocess_plugin(script.to_str().unwrap(), b"env-content", &PluginOptions::new()).unwrap();
        assert_eq!(out, b"env-content");
    }
}
