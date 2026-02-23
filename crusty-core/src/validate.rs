//! Orchestration validation: type intersection rule (Output(A) ∩ Input(B) ≠ ∅).

use crate::orchestration::Orchestration;
use crate::plugin::ManifestCapabilities;
use std::path::Path;

fn types_intersect(a: &[String], b: &[String]) -> bool {
    a.iter().any(|t| b.contains(t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn types_intersect_yes() {
        let a = vec!["text/plain".to_string(), "text".to_string()];
        let b = vec!["text".to_string()];
        assert!(types_intersect(&a, &b));
    }

    #[test]
    fn types_intersect_no() {
        let a = vec!["audio/raw".to_string()];
        let b = vec!["text/plain".to_string()];
        assert!(!types_intersect(&a, &b));
    }

    #[test]
    fn validate_orchestration_types_ok_with_manifests() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let tts_dir = base.join("tts-plugin");
        fs::create_dir_all(&tts_dir).unwrap();
        fs::write(
            tts_dir.join("plugin.toml"),
            r#"
name = "tts"
version = "0.1"
type = "tts"
[capabilities]
input = ["text/plain", "text"]
output = ["audio/wav"]
"#,
        )
        .unwrap();
        let orch_toml = r#"
[meta]
name = "t"
version = "0.1"
author = "a"
[input]
type = "text"
source = "in.txt"
[tts]
name = "tts"
module = "tts-plugin"
[output]
type = "file"
path = "out.bin"
"#;
        let orch = Orchestration::from_toml(orch_toml).unwrap();
        assert!(validate_orchestration_types(&orch, base).is_ok());
    }

    #[test]
    fn validate_orchestration_types_mismatch_fails() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let tts_dir = base.join("tts-plugin");
        fs::create_dir_all(&tts_dir).unwrap();
        // TTS declares only audio input - pipeline sends text, so no intersection
        fs::write(
            tts_dir.join("plugin.toml"),
            r#"
name = "tts"
version = "0.1"
type = "tts"
[capabilities]
input = ["audio/raw"]
output = ["audio/wav"]
"#,
        )
        .unwrap();
        let orch_toml = r#"
[meta]
name = "t"
version = "0.1"
author = "a"
[input]
type = "text"
source = "in.txt"
[tts]
name = "tts"
module = "tts-plugin"
[output]
type = "file"
path = "out.bin"
"#;
        let orch = Orchestration::from_toml(orch_toml).unwrap();
        let err = validate_orchestration_types(&orch, base).unwrap_err();
        assert!(err.to_string().contains("does not match input"));
    }
}

fn load_manifest_capabilities(plugin_base: &Path, module: &str) -> Option<ManifestCapabilities> {
    let toml_path = plugin_base.join(module).join("plugin.toml");
    let s = std::fs::read_to_string(toml_path).ok()?;
    let manifest: crate::plugin::PluginManifest = toml::from_str(&s).ok()?;
    manifest.capabilities
}

/// Default output type for pipeline start (input is text).
const INPUT_TEXT: &[&str] = &["text/plain", "text"];

/// Validate that for each adjacent pair of stages, output types of stage N
/// intersect input types of stage N+1. If a manifest does not declare
/// input/output, that link is skipped (no type check).
pub fn validate_orchestration_types(
    orch: &Orchestration,
    plugin_base: &Path,
) -> anyhow::Result<()> {
    let mut prev_output: Vec<String> = INPUT_TEXT.iter().map(|s| s.to_string()).collect();

    // Pre-processors: input text → output text
    if let Some(ref pre) = orch.pre_processors {
        for p in pre.iter().filter(|p| p.enabled) {
            let cap = load_manifest_capabilities(plugin_base, &p.module);
            let input = cap.as_ref().and_then(|c| c.input.as_ref()).map(|v| v.clone());
            let output = cap.as_ref().and_then(|c| c.output.as_ref()).map(|v| v.clone());
            if let Some(ref inp) = input {
                if !types_intersect(&prev_output, inp) {
                    anyhow::bail!(
                        "pre-processor {}: pipeline output {:?} does not match input {:?}",
                        p.name,
                        prev_output,
                        inp
                    );
                }
            }
            prev_output = output.unwrap_or_else(|| vec!["text/plain".to_string()]);
        }
    }

    // TTS: text → audio
    let cap = load_manifest_capabilities(plugin_base, &orch.tts.module);
    let tts_input = cap.as_ref().and_then(|c| c.input.as_ref()).map(|v| v.clone());
    let tts_output = cap.as_ref().and_then(|c| c.output.as_ref()).map(|v| v.clone());
    if let Some(ref inp) = tts_input {
        if !types_intersect(&prev_output, inp) {
            anyhow::bail!(
                "TTS {}: pipeline output {:?} does not match input {:?}",
                orch.tts.name,
                prev_output,
                inp
            );
        }
    }
    prev_output = tts_output.unwrap_or_else(|| vec!["audio/raw".to_string(), "audio/wav".to_string()]);

    // Converters
    if let Some(ref conv) = orch.audio_converters {
        for node in conv.iter().filter(|c| c.enabled) {
            let cap = load_manifest_capabilities(plugin_base, &node.module);
            let input = cap.as_ref().and_then(|c| c.input.as_ref()).map(|v| v.clone());
            let output = cap.as_ref().and_then(|c| c.output.as_ref()).map(|v| v.clone());
            if let Some(ref inp) = input {
                if !types_intersect(&prev_output, inp) {
                    anyhow::bail!(
                        "converter {}: pipeline output {:?} does not match input {:?}",
                        node.name,
                        prev_output,
                        inp
                    );
                }
            }
            prev_output = output.unwrap_or_else(|| vec!["audio/raw".to_string()]);
        }
    }

    // Post-processors
    if let Some(ref post) = orch.post_processors {
        for p in post.iter().filter(|p| p.enabled) {
            let cap = load_manifest_capabilities(plugin_base, &p.module);
            let input = cap.as_ref().and_then(|c| c.input.as_ref()).map(|v| v.clone());
            if let Some(ref inp) = input {
                if !types_intersect(&prev_output, inp) {
                    anyhow::bail!(
                        "post-processor {}: pipeline output {:?} does not match input {:?}",
                        p.name,
                        prev_output,
                        inp
                    );
                }
            }
        }
    }

    Ok(())
}
