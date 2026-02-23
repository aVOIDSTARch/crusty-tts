

Skip to content
Using Casinelli LLC Mail with screen readers
2 of 28
plan
External
Inbox
Summarize this email
Lou Casinelli

Attachments6:52 PM (7 minutes ago)


to me
 One attachment
  •  Scanned by Gmail

Crusty-TTS Architecture & Design Supplement

Overview

Crusty-TTS is a modular text-to-speech orchestration library designed to allow flexible plugin integration, pre/post-processing, and audio format conversion. It supports multiple interfaces (CLI, GUI, web, app) via a developer-facing API.

⸻

Core Concepts

Plugin System
	•	Plugins live in /plugins/<plugin_name> or can be pulled via URL.
	•	Each plugin declares capabilities in a plugin.toml:
	•	pre_processor (optional)
	•	synthesizer (mandatory for TTS plugins)
	•	post_processor (optional)
	•	audio_formats supported
	•	requirements for runtime dependencies
	•	Plugins must implement a verification function:
	•	Minimal test of input/output
	•	Security checks (optional sandboxing)
	•	TOML validation

Meta-Processor
	•	Handles pre-processing and post-processing tasks.
	•	Single interface abstracts plugin type (pre/post/both).
	•	Core orchestration node manages registration, verification, and pipeline execution.

⸻

Pipeline Orchestration

Flow:
Pre-processor -> TTS Synthesizer -> Audio Converter (MP3/other) -> Post-processor
	•	Configuration stored in orchestration.cr (or .or for MIME awareness).
	•	Interactive CLI/GUI generates orchestration.cr from plugin TOMLs.
	•	Supports multiple audio formats per run.
	•	Allows custom pre/post processing chains.

⸻

Interface Design
	•	Core library exposes API, not UI.
	•	Developers may build:
	•	CLI
	•	Web portal
	•	Native apps
	•	Browser extensions
	•	API allows querying plugins, capabilities, and orchestrating runs.

⸻

Code Execution Strategy
	•	Each plugin runs in an isolated subprocess (code-agnostic approach).
	•	Dynamic requirements reading from TOML for plugin dependencies.
	•	MP3 or other audio conversions done by separate plugin interface.
	•	Extensible to support new TTS engines, audio formats, or text processing tools.

⸻

File Structure (Example)

### code
crusty-tts/
├─ src/
│  ├─ core.rs           # Orchestration engine
│  ├─ plugin_manager.rs # Plugin registration/verification
│  ├─ pipeline.rs       # Pipeline execution
├─ plugins/
│  ├─ myplugin/
│  │  ├─ plugin.rs
│  │  ├─ plugin.toml
├─ orchestration.cr      # Generated configuration
├─ Cargo.toml
### end code


⸻

Plugin TOML Example
### code
name = "example_tts"
version = "0.1.0"
pre_processor = true
synthesizer = true
post_processor = false
audio_formats = ["mp3", "wav"]
requirements = ["numpy", "soundfile"]
description = "Example TTS plugin supporting pre-processing and multiple audio formats."

### end code


⸻

Design Decisions
	•	Flexible plugin system: allows future expansion and developer autonomy.
	•	Isolated execution: protects core from plugin failures or malicious code.
	•	Pipeline-based orchestration: pre->synth->post allows maximum modularity.
	•	Core API: UI is separate to encourage multi-platform interfaces.
	•	Standardized plugin declaration: TOML provides clear, structured metadata.
	•	Interactive configuration: simplifies user setup without losing advanced control.

⸻

Next Steps
	1.	Implement plugin manager & verification.
	2.	Build pipeline orchestration node.
	3.	Create example TTS plugin & pre/post processors.
	4.	Develop interactive CLI for orchestration.cr generation.
	5.	Extend to support audio format plugins.
	6.	Optional: GUI API layer for developer interface.

plan.md
Displaying plan.md.
