//! Integration tests: full pipeline with temp fixtures.

use crusty_core::{execute_pipeline, validate_orchestration_types, Orchestration, PluginRegistry};
use std::fs;

#[cfg(unix)]
#[test]
fn execute_pipeline_with_stub_plugin() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();

    // input
    fs::write(base.join("input.txt"), "Hello world").unwrap();

    // plugin: tts stub (echo stdin to stdout via env)
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
    fs::set_permissions(&run_sh, fs::Permissions::from_mode(0o755)).unwrap();

    let orch_toml = format!(
        r#"
[meta]
name = "test"
version = "0.1"
author = "test"
[input]
type = "text"
source = "{}"
[tts]
name = "tts-stub"
module = "plugins/tts-stub"
[output]
type = "file"
path = "out.bin"
"#,
        base.join("input.txt").display()
    );
    let orch = Orchestration::from_toml(&orch_toml).unwrap();
    assert!(validate_orchestration_types(&orch, base).is_ok());

    let out = execute_pipeline(&orch, base).unwrap();
    assert_eq!(out, b"Hello world");
}

#[test]
fn registry_and_validate_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    let plugins_base = base.join("plugins");
    let my_tts_dir = plugins_base.join("my-tts");
    fs::create_dir_all(&my_tts_dir).unwrap();
    fs::write(
        my_tts_dir.join("plugin.toml"),
        r#"
name = "my-tts"
version = "0.1"
type = "tts"
[capabilities]
input = ["text/plain"]
output = ["audio/wav"]
"#,
    )
    .unwrap();

    let reg = PluginRegistry::load_plugins(&plugins_base).unwrap();
    assert_eq!(reg.tts.len(), 1);
    assert_eq!(reg.get("my-tts").unwrap().name, "my-tts");

    let orch_toml = format!(
        r#"
[meta]
name = "t"
version = "0.1"
author = "a"
[input]
type = "text"
source = "{}"
[tts]
name = "my-tts"
module = "plugins/my-tts"
[output]
type = "file"
path = "out.bin"
"#,
        base.join("in.txt").display()
    );
    fs::write(base.join("in.txt"), "x").unwrap();
    let orch = Orchestration::from_toml(&orch_toml).unwrap();
    assert!(validate_orchestration_types(&orch, base).is_ok());
}
