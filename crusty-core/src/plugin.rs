//! Plugin types: manifest (plugin.toml), plugin type enum, and optional in-process traits.

use serde::Deserialize;
use std::collections::HashMap;

pub type PluginOptions = HashMap<String, String>;

/// Plugin role in the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    Pre,
    Tts,
    Post,
    Converter,
}

impl PluginType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginType::Pre => "pre",
            PluginType::Tts => "tts",
            PluginType::Post => "post",
            PluginType::Converter => "converter",
        }
    }
}

/// Discovered plugin: path + manifest-derived info.
#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub plugin_type: PluginType,
    /// Directory path containing plugin.toml and entrypoint.
    pub path: String,
    pub options: PluginOptions,
    pub manifest: Option<PluginManifest>,
}

/// Parsed plugin.toml (capabilities, options schema).
#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(alias = "protocol_version", default)]
    pub api_version: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub capabilities: Option<ManifestCapabilities>,
    #[serde(default)]
    pub options: Option<toml::Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ManifestCapabilities {
    #[serde(default)]
    pub input: Option<Vec<String>>,
    #[serde(default)]
    pub output: Option<Vec<String>>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub preprocessor: Option<bool>,
    #[serde(default)]
    pub tts: Option<bool>,
    #[serde(default)]
    pub postprocessor: Option<bool>,
    #[serde(default)]
    pub output_formats: Option<Vec<String>>,
}

// --- Optional in-process traits (for native Rust plugins) ---

pub trait PreProcessor {
    fn name(&self) -> &str;
    fn process(&self, input: &str, options: &PluginOptions) -> String;
}

pub trait Tts {
    fn name(&self) -> &str;
    fn synthesize(&self, input: &str, options: &PluginOptions) -> Vec<u8>;
}

pub trait PostProcessor {
    fn name(&self) -> &str;
    fn process(&self, input: &[u8], options: &PluginOptions) -> Vec<u8>;
}
