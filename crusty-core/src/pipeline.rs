//! Pipeline execution: pre -> TTS -> converter -> post using subprocess runner.

use crate::orchestration::Orchestration;
use crate::plugin::{PluginOptions, PluginType};
use crate::plugin_runner::run_subprocess_plugin;
use std::path::Path;

/// Resolved plugin executable path (e.g. plugins/name/run.sh or entrypoint from manifest).
fn plugin_executable(plugin_dir: &str, _manifest: Option<&crate::plugin::PluginManifest>) -> Option<std::path::PathBuf> {
    let run_sh = Path::new(plugin_dir).join("run.sh");
    if run_sh.exists() {
        return Some(run_sh);
    }
    let run_py = Path::new(plugin_dir).join("run.py");
    if run_py.exists() {
        return Some(run_py);
    }
    None
}

/// Execute full orchestration: load input, run pre -> tts -> converters -> post, write output.
pub fn execute_pipeline(orchestration: &Orchestration, plugin_base_dir: &Path) -> anyhow::Result<Vec<u8>> {
    let input_path = Path::new(&orchestration.input.source);
    let mut text = std::fs::read_to_string(input_path)
        .map_err(|e| anyhow::anyhow!("read input {:?}: {}", input_path, e))?;

    // Pre-processors
    if let Some(ref pre) = orchestration.pre_processors {
        for p in pre.iter().filter(|p| p.enabled) {
            let plugin_dir = plugin_base_dir.join(&p.module);
            let exec = plugin_executable(plugin_dir.to_str().unwrap(), None)
                .ok_or_else(|| anyhow::anyhow!("no executable for pre-processor {}", p.name))?;
            let opts = options_from_toml(p.options.as_ref());
            let out = run_subprocess_plugin(exec.to_str().unwrap(), text.as_bytes(), &opts)?;
            text = String::from_utf8(out).map_err(|_| anyhow::anyhow!("pre-processor must return UTF-8 text"))?;
        }
    }

    // TTS
    let plugin_dir = plugin_base_dir.join(&orchestration.tts.module);
    let exec = plugin_executable(plugin_dir.to_str().unwrap(), None)
        .ok_or_else(|| anyhow::anyhow!("no executable for TTS {}", orchestration.tts.name))?;
    let mut opts = PluginOptions::new();
    if let Some(v) = &orchestration.tts.voice {
        opts.insert("voice".into(), v.clone());
    }
    if let Some(r) = orchestration.tts.rate {
        opts.insert("rate".into(), r.to_string());
    }
    if let Some(p) = orchestration.tts.pitch {
        opts.insert("pitch".into(), p.to_string());
    }
    let mut audio = run_subprocess_plugin(exec.to_str().unwrap(), text.as_bytes(), &opts)?;

    // Audio converters
    if let Some(ref conv) = orchestration.audio_converters {
        for c in conv.iter().filter(|c| c.enabled) {
            let plugin_dir = plugin_base_dir.join(&c.module);
            let exec = plugin_executable(plugin_dir.to_str().unwrap(), None)
                .ok_or_else(|| anyhow::anyhow!("no executable for converter {}", c.name))?;
            let opts = options_from_toml(c.options.as_ref());
            audio = run_subprocess_plugin(exec.to_str().unwrap(), &audio, &opts)?;
        }
    }

    // Post-processors
    if let Some(ref post) = orchestration.post_processors {
        for p in post.iter().filter(|p| p.enabled) {
            let plugin_dir = plugin_base_dir.join(&p.module);
            let exec = plugin_executable(plugin_dir.to_str().unwrap(), None)
                .ok_or_else(|| anyhow::anyhow!("no executable for post-processor {}", p.name))?;
            let opts = options_from_toml(p.options.as_ref());
            audio = run_subprocess_plugin(exec.to_str().unwrap(), &audio, &opts)?;
        }
    }

    Ok(audio)
}

fn options_from_toml(v: Option<&toml::Value>) -> PluginOptions {
    let mut opts = PluginOptions::new();
    let Some(tbl) = v.and_then(|v| v.as_table()) else { return opts };
    for (k, v) in tbl {
        let s = match v {
            toml::Value::String(s) => s.clone(),
            toml::Value::Integer(i) => i.to_string(),
            toml::Value::Float(f) => f.to_string(),
            toml::Value::Boolean(b) => b.to_string(),
            _ => continue,
        };
        opts.insert(k.clone(), s);
    }
    opts
}

/// Run pipeline from discovered plugins (by type order: pre, tts, post/converter).
pub fn run_pipeline_from_plugins(input_text: &str, plugins: &[crate::plugin::Plugin]) -> anyhow::Result<Vec<u8>> {
    let mut text = input_text.to_string();
    let mut audio: Vec<u8> = vec![];

    for p in plugins.iter().filter(|p| p.plugin_type == PluginType::Pre) {
        let exec = plugin_executable(&p.path, p.manifest.as_ref())
            .ok_or_else(|| anyhow::anyhow!("no executable for {}", p.path))?;
        audio = run_subprocess_plugin(exec.to_str().unwrap(), text.as_bytes(), &p.options)?;
        text = String::from_utf8(audio.clone()).map_err(|_| anyhow::anyhow!("pre-plugin must return UTF-8 text"))?;
    }

    for p in plugins.iter().filter(|p| p.plugin_type == PluginType::Tts) {
        let exec = plugin_executable(&p.path, p.manifest.as_ref())
            .ok_or_else(|| anyhow::anyhow!("no executable for {}", p.path))?;
        audio = run_subprocess_plugin(exec.to_str().unwrap(), text.as_bytes(), &p.options)?;
    }

    for p in plugins.iter().filter(|p| p.plugin_type == PluginType::Post || p.plugin_type == PluginType::Converter) {
        let exec = plugin_executable(&p.path, p.manifest.as_ref())
            .ok_or_else(|| anyhow::anyhow!("no executable for {}", p.path))?;
        audio = run_subprocess_plugin(exec.to_str().unwrap(), &audio, &p.options)?;
    }

    Ok(audio)
}
