//! Crusty-TTS CLI: run pipeline, or interactive configure to write orchestration.cr.

use anyhow::Result;
use crusty_core::{
    execute_pipeline, Orchestration, PluginRegistry, PluginType,
    orchestration::{Meta, Input, Output, PluginConfig, TtsConfig},
};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("configure") {
        run_configure(&args[2..])?;
        return Ok(());
    }
    run_pipeline(&args)?;
    Ok(())
}

fn run_configure(args: &[String]) -> Result<()> {
    let plugins_dir = args
        .iter()
        .position(|a| a == "--plugins" || a == "-p")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("plugins"));

    let registry = PluginRegistry::load_plugins(&plugins_dir)?;
    let all: Vec<_> = registry.all().into_iter().cloned().collect();

    if all.is_empty() {
        eprintln!("No plugins found in {}", plugins_dir.display());
        return Ok(());
    }

    eprintln!("Found {} plugins:", all.len());
    for (i, p) in all.iter().enumerate() {
        eprintln!("  {}. {} ({})", i + 1, p.name, p.plugin_type.as_str());
    }

    eprint!("Select plugins for pipeline (comma-separated, e.g. 1,2): ");
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let indices: Vec<usize> = line
        .trim()
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok().map(|n| n.saturating_sub(1)))
        .collect();

    let mut pre = Vec::new();
    let mut tts_plugin = None;
    let mut converters = Vec::new();
    let mut post = Vec::new();

    for &idx in &indices {
        let Some(p) = all.get(idx) else { continue };
        match p.plugin_type {
            PluginType::Pre => pre.push(p.clone()),
            PluginType::Tts => {
                if tts_plugin.is_none() {
                    tts_plugin = Some(p.clone());
                }
            }
            PluginType::Converter => converters.push(p.clone()),
            PluginType::Post => post.push(p.clone()),
        }
    }

    let tts_plugin = match tts_plugin {
        Some(p) => p,
        None => {
            eprintln!("No TTS plugin selected. Select at least one TTS.");
            return Ok(());
        }
    };

    // Prompt for options per selected plugin
    let pre_configs: Vec<PluginConfig> = pre
        .iter()
        .map(|p| prompt_plugin_options(p, &plugins_dir))
        .collect();
    let tts_config = plugin_to_tts_config(&tts_plugin, &plugins_dir);
    let conv_configs: Vec<PluginConfig> = converters
        .iter()
        .map(|p| prompt_plugin_options(p, &plugins_dir))
        .collect();
    let post_configs: Vec<PluginConfig> = post
        .iter()
        .map(|p| prompt_plugin_options(p, &plugins_dir))
        .collect();

    let orchestration = Orchestration {
        meta: Meta {
            name: "configured-pipeline".to_string(),
            version: "0.1.0".to_string(),
            author: "Crusty-TTS".to_string(),
        },
        input: Input {
            r#type: "text".to_string(),
            source: "input.txt".to_string(),
        },
        pre_processors: if pre_configs.is_empty() {
            None
        } else {
            Some(pre_configs)
        },
        tts: tts_config,
        audio_converters: if conv_configs.is_empty() {
            None
        } else {
            Some(conv_configs)
        },
        post_processors: if post_configs.is_empty() {
            None
        } else {
            Some(post_configs)
        },
        output: Output {
            r#type: "file".to_string(),
            path: "output/out.bin".to_string(),
            overwrite: Some(true),
        },
    };

    let toml = toml::to_string_pretty(&orchestration)?;
    let out_path = PathBuf::from("orchestration.cr");
    std::fs::write(&out_path, toml)?;
    eprintln!("Pipeline orchestration saved to {}", out_path.display());
    Ok(())
}

fn prompt_plugin_options(p: &crusty_core::Plugin, _plugins_dir: &Path) -> PluginConfig {
    let path = Path::new(&p.path);
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let module = path
        .strip_prefix(&cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let mut options = toml::value::Table::new();
    for (k, v) in &p.options {
        eprint!("  {} [{}] for {}: ", k, v, p.name);
        io::stderr().flush().ok();
        let mut line = String::new();
        io::stdin().read_line(&mut line).ok();
        let val = line.trim();
        let value = if val.is_empty() {
            toml::Value::String(v.clone())
        } else {
            toml::Value::String(val.to_string())
        };
        options.insert(k.clone(), value);
    }
    PluginConfig {
        name: p.name.clone(),
        module,
        enabled: true,
        options: Some(toml::Value::Table(options)),
    }
}

fn plugin_to_tts_config(p: &crusty_core::Plugin, _plugins_dir: &Path) -> TtsConfig {
    let path = Path::new(&p.path);
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let module = path
        .strip_prefix(&cwd)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    TtsConfig {
        name: p.name.clone(),
        module,
        voice: p.options.get("voice").cloned(),
        rate: p.options.get("rate").and_then(|s| s.parse().ok()),
        pitch: p.options.get("pitch").and_then(|s| s.parse().ok()),
        output_format: Some("wav".to_string()),
    }
}

fn run_pipeline(args: &[String]) -> Result<()> {
    let (orchestration_path, plugin_dir, input_override, output_override) = parse_args(args)?;

    let mut orchestration = Orchestration::load_path(&orchestration_path)?;
    let plugin_base = plugin_dir.unwrap_or_else(|| {
        orchestration_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    });
    let input_path = PathBuf::from(
        input_override.unwrap_or_else(|| orchestration.input.source.clone())
    );
    if input_path.is_relative() {
        let base = orchestration_path.parent().unwrap_or_else(|| Path::new("."));
        orchestration.input.source = base.join(input_path).to_string_lossy().to_string();
    } else {
        orchestration.input.source = input_path.to_string_lossy().to_string();
    }

    let audio = execute_pipeline(&orchestration, &plugin_base)?;

    match output_override.as_deref() {
        Some("-") => {
            io::stdout().write_all(&audio)?;
            io::stdout().flush()?;
        }
        Some(path) => {
            let out_path = PathBuf::from(path);
            if let Some(p) = out_path.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::write(path, &audio)?;
            eprintln!("Wrote {} bytes to {}", audio.len(), path);
        }
        None => {
            let out_path = PathBuf::from(&orchestration.output.path);
            if let Some(p) = out_path.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::write(&orchestration.output.path, &audio)?;
            eprintln!("Wrote {} bytes to {}", audio.len(), orchestration.output.path);
        }
    }
    Ok(())
}

fn parse_args(
    args: &[String],
) -> Result<(PathBuf, Option<PathBuf>, Option<String>, Option<String>)> {
    let mut orch = PathBuf::from("orchestration.cr");
    let mut plugin_dir = None;
    let mut input_override = None;
    let mut output_override = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "run" => {
                i += 1;
                if i < args.len() {
                    orch = PathBuf::from(&args[i]);
                    i += 1;
                }
            }
            "--orchestration" | "-o" => {
                i += 1;
                if i < args.len() {
                    orch = PathBuf::from(&args[i]);
                    i += 1;
                }
            }
            "--plugins" | "-p" => {
                i += 1;
                if i < args.len() {
                    plugin_dir = Some(PathBuf::from(&args[i]));
                    i += 1;
                }
            }
            "--input" | "-i" => {
                i += 1;
                if i < args.len() {
                    input_override = Some(args[i].clone());
                    i += 1;
                }
            }
            "--output" => {
                i += 1;
                if i < args.len() {
                    output_override = Some(args[i].clone());
                    i += 1;
                }
            }
            _ => {
                if orch == PathBuf::from("orchestration.cr") && !args[i].starts_with('-') {
                    orch = PathBuf::from(&args[i]);
                }
                i += 1;
            }
        }
    }
    Ok((orch, plugin_dir, input_override, output_override))
}
