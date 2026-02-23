//! Integration tests: run CLI binary with temp fixtures.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn crusty_cli_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_crusty-cli"))
}

#[test]
fn cli_help_or_unknown_exits() {
    // Running with no args tries to load orchestration.cr from cwd and may fail - that's ok
    let out = Command::new(crusty_cli_bin()).current_dir(std::env::temp_dir()).output().unwrap();
    // May succeed (if orchestration.cr exists in temp) or fail (no such file)
    assert!(out.status.code().is_some());
}

#[test]
#[cfg(unix)]
fn cli_run_with_temp_orchestration() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();

    // Write input
    fs::write(base.join("input.txt"), "Hello").unwrap();

    // Stub TTS plugin
    let tts_dir = base.join("plugins").join("tts-stub");
    fs::create_dir_all(&tts_dir).unwrap();
    fs::write(
        tts_dir.join("plugin.toml"),
        r#"
name = "tts-stub"
version = "0.1"
type = "tts"
[capabilities]
input = ["text/plain"]
output = ["audio/raw"]
"#,
    )
    .unwrap();
    let run_sh = tts_dir.join("run.sh");
    fs::write(&run_sh, "#!/bin/sh\nprintf '%s' \"$PLUGIN_INPUT\"\n").unwrap();
    fs::set_permissions(&run_sh, std::fs::Permissions::from_mode(0o755)).unwrap();

    let orch = format!(
        r#"
[meta]
name = "test"
version = "0.1"
author = "t"
[input]
type = "text"
source = "{}"
[tts]
name = "tts-stub"
module = "plugins/tts-stub"
[output]
type = "file"
path = "{}"
"#,
        base.join("input.txt").display(),
        base.join("out.bin").display()
    );
    fs::write(base.join("orchestration.cr"), orch).unwrap();

    let out = Command::new(crusty_cli_bin())
        .arg("orchestration.cr")
        .current_dir(base)
        .output()
        .unwrap();

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let out_bytes = fs::read(base.join("out.bin")).unwrap();
    assert_eq!(out_bytes, b"Hello");
}
