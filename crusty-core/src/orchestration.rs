//! Orchestration file types: meta, input, plugins, tts, output.
//! Supports both Foldedbits-style (meta/input/pre_processors/tts/...) and pipeline-order style.

use serde::{Deserialize, Serialize};

/// Full orchestration config (orchestration.cr).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Orchestration {
    pub meta: Meta,
    pub input: Input,
    #[serde(default)]
    pub pre_processors: Option<Vec<PluginConfig>>,
    pub tts: TtsConfig,
    #[serde(default)]
    pub audio_converters: Option<Vec<PluginConfig>>,
    #[serde(default)]
    pub post_processors: Option<Vec<PluginConfig>>,
    pub output: Output,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Meta {
    pub name: String,
    pub version: String,
    pub author: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Input {
    pub r#type: String,
    pub source: String,
}

/// Per-plugin entry in orchestration (pre, converter, post).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginConfig {
    pub name: String,
    pub module: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub options: Option<toml::Value>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TtsConfig {
    pub name: String,
    pub module: String,
    pub voice: Option<String>,
    pub rate: Option<f32>,
    pub pitch: Option<f32>,
    pub output_format: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Output {
    pub r#type: String,
    pub path: String,
    #[serde(default)]
    pub overwrite: Option<bool>,
}

/// Alternative format: pipeline order + plugin options (from interactive CLI).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipelineOrchestration {
    pub pipeline: PipelineSection,
    #[serde(flatten)]
    pub plugin_options: std::collections::HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipelineSection {
    pub order: Vec<String>,
}

impl Orchestration {
    /// Load from TOML string.
    pub fn from_toml(s: &str) -> anyhow::Result<Self> {
        let o: Orchestration = toml::from_str(s)?;
        Ok(o)
    }

    /// Load from file path.
    pub fn load_path(path: &std::path::Path) -> anyhow::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        Self::from_toml(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_ORCH: &str = r#"
[meta]
name = "test"
version = "0.1.0"
author = "Test"

[input]
type = "text"
source = "input.txt"

[tts]
name = "tts"
module = "plugins/tts"

[output]
type = "file"
path = "out.bin"
"#;

    #[test]
    fn from_toml_minimal() {
        let o = Orchestration::from_toml(MINIMAL_ORCH).unwrap();
        assert_eq!(o.meta.name, "test");
        assert_eq!(o.input.source, "input.txt");
        assert_eq!(o.tts.name, "tts");
        assert_eq!(o.output.path, "out.bin");
        assert!(o.pre_processors.is_none());
        assert!(o.audio_converters.is_none());
        assert!(o.post_processors.is_none());
    }

    #[test]
    fn from_toml_with_pre_and_post() {
        let s = r#"
[meta]
name = "full"
version = "0.1.0"
author = "A"

[input]
type = "text"
source = "in.txt"

[[pre_processors]]
name = "pre"
module = "plugins/pre"
enabled = true

[tts]
name = "tts"
module = "plugins/tts"
voice = "en"

[[post_processors]]
name = "post"
module = "plugins/post"

[output]
type = "file"
path = "out.mp3"
"#;
        let o = Orchestration::from_toml(s).unwrap();
        assert_eq!(o.pre_processors.as_ref().unwrap().len(), 1);
        assert_eq!(o.pre_processors.as_ref().unwrap()[0].name, "pre");
        assert_eq!(o.post_processors.as_ref().unwrap().len(), 1);
        assert_eq!(o.tts.voice.as_deref(), Some("en"));
    }

    #[test]
    fn from_toml_invalid_fails() {
        assert!(Orchestration::from_toml("invalid = [").is_err());
        assert!(Orchestration::from_toml("[meta]\nname = 1").is_err());
    }

    #[test]
    fn pipeline_orchestration_parse() {
        let s = r#"
[pipeline]
order = ["A", "B"]

[A.options]
x = "1"
"#;
        let o: super::PipelineOrchestration = toml::from_str(s).unwrap();
        assert_eq!(o.pipeline.order, &["A", "B"]);
    }
}
