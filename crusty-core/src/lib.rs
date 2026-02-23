//! Crusty-TTS core: orchestration, plugin registry, pipeline execution, protocol.

pub mod orchestration;
pub mod pipeline;
pub mod plugin;
pub mod plugin_runner;
pub mod protocol;
pub mod registry;
pub mod validate;

pub use orchestration::{Orchestration, Output, PipelineOrchestration, PipelineSection, PluginConfig, TtsConfig};
pub use pipeline::{execute_pipeline, run_pipeline_from_plugins};
pub use plugin::{Plugin, PluginManifest, PluginOptions, PluginType, PostProcessor, PreProcessor, Tts};
pub use plugin_runner::{run_subprocess_plugin, run_subprocess_plugin_framed, verify_plugin};
pub use protocol::{Handshake, ErrorFrame, PROTOCOL_VERSION};
pub use registry::PluginRegistry;
pub use validate::validate_orchestration_types;
