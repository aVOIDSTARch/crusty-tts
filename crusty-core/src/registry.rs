//! Plugin discovery: load plugin.toml from /plugins, build registry.

use crate::plugin::{Plugin, PluginManifest, PluginOptions, PluginType};
use std::collections::HashMap;
use std::path::Path;

/// Registry of discovered plugins by type.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    pub pre: Vec<Plugin>,
    pub tts: Vec<Plugin>,
    pub post: Vec<Plugin>,
    pub converter: Vec<Plugin>,
    /// All by name for lookup.
    by_name: HashMap<String, Plugin>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_plugins(plugin_dir: &Path) -> anyhow::Result<Self> {
        let mut reg = PluginRegistry::new();
        for entry in std::fs::read_dir(plugin_dir)? {
            let entry = entry?;
            if !entry.path().is_dir() {
                continue;
            }
            let toml_path = entry.path().join("plugin.toml");
            if !toml_path.exists() {
                continue;
            }
            let contents = std::fs::read_to_string(&toml_path)?;
            let manifest: PluginManifest = toml::from_str(&contents)?;
            let plugin_type = manifest
                .r#type
                .as_deref()
                .or_else(|| manifest.capabilities.as_ref().and_then(|c| {
                    if c.tts == Some(true) {
                        Some("tts")
                    } else if c.preprocessor == Some(true) {
                        Some("pre")
                    } else if c.postprocessor == Some(true) {
                        Some("post")
                    } else {
                        None
                    }
                }))
                .unwrap_or("pre");

            let plugin_type = match plugin_type {
                "pre" | "preprocessor" => PluginType::Pre,
                "tts" | "synth" => PluginType::Tts,
                "post" | "postprocessor" => PluginType::Post,
                "converter" | "encode" => PluginType::Converter,
                _ => PluginType::Pre,
            };

            let mut options = PluginOptions::new();
            if let Some(ref opts) = manifest.options {
                if let Some(tbl) = opts.as_table() {
                    for (k, v) in tbl {
                        let val = v.as_table().and_then(|t| t.get("default")).unwrap_or(v);
                        let s = match val {
                            toml::Value::String(s) => s.clone(),
                            toml::Value::Integer(i) => i.to_string(),
                            toml::Value::Float(f) => f.to_string(),
                            toml::Value::Boolean(b) => b.to_string(),
                            _ => continue,
                        };
                        options.insert(k.clone(), s);
                    }
                }
            }

            let path = entry.path().to_string_lossy().to_string();
            let plugin = Plugin {
                name: manifest.name.clone(),
                plugin_type,
                path: path.clone(),
                options,
                manifest: Some(manifest),
            };
            reg.by_name.insert(plugin.name.clone(), plugin.clone());
            match plugin.plugin_type {
                PluginType::Pre => reg.pre.push(plugin),
                PluginType::Tts => reg.tts.push(plugin),
                PluginType::Post => reg.post.push(plugin),
                PluginType::Converter => reg.converter.push(plugin),
            }
        }
        Ok(reg)
    }

    pub fn get(&self, name: &str) -> Option<&Plugin> {
        self.by_name.get(name)
    }

    pub fn all(&self) -> Vec<&Plugin> {
        self.by_name.values().collect()
    }

    /// Ordered list for pipeline: pre, tts, converter, post.
    pub fn pipeline_order(&self, order: &[String]) -> Vec<Plugin> {
        order
            .iter()
            .filter_map(|n| self.by_name.get(n).cloned())
            .collect()
    }
}
