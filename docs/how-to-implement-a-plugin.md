# How to Implement a Crusty-TTS Plugin

This guide describes how to implement a plugin that works with Crusty-TTS **without reading crusty-core source**. Rely on the [Plugin Protocol Spec](plugin-protocol-spec-v0.1.md) and this document.

## 1. Plugin layout

Create a directory under `plugins/` (or your configured plugin dir):

```
plugins/my-plugin/
  plugin.toml    # required: manifest
  run.sh         # or run.py, or any executable (entrypoint)
```

## 2. plugin.toml (manifest)

Required fields:

- **name** — Plugin name (unique in the registry).
- **version** — Semver string.
- **type** — One of: `pre`, `tts`, `post`, `converter`.
- **capabilities** (optional but recommended for validation):
  - **input** — List of MIME-style input types (e.g. `["text/plain", "text"]`).
  - **output** — List of output types (e.g. `["audio/raw", "audio/wav"]`).
  - **mode** — `"streaming"` or `"batch"`.

Optional:

- **options** — Schema for plugin options so Crusty can build UIs:
  - `[options.voice] type = "string" default = "en_us"`
  - `[options.rate] type = "float" default = 1.0`

Example:

```toml
name = "my-tts"
version = "0.1.0"
type = "tts"

[capabilities]
input = ["text/plain"]
output = ["audio/wav"]
mode = "streaming"

[options]
voice = { type = "string", default = "en_us" }
rate = { type = "float", default = 1.0 }
```

## 3. Execution contract (simple / env-based)

Crusty can run your plugin in two ways:

### 3.1 Env-based (recommended for v1)

- Crusty sets **PLUGIN_INPUT** (UTF-8 string: text or path).
- For each option, Crusty sets **PLUGIN_OPT_&lt;NAME&gt;** (e.g. `PLUGIN_OPT_VOICE`, `PLUGIN_OPT_RATE`).
- Your script reads stdin (optional) and **writes output to stdout** (raw bytes: text for pre, audio for tts/post/converter).

Example (Bash):

```bash
INPUT="${PLUGIN_INPUT:-}"
VOICE="${PLUGIN_OPT_VOICE:-en_us}"
# ... generate audio from INPUT ...
# write raw audio to stdout
cat generated.wav
```

Example (Python):

```python
import os, sys
input_text = os.environ.get("PLUGIN_INPUT", "")
sys.stdout.buffer.write(my_tts_synthesize(input_text))
sys.stdout.buffer.flush()
```

### 3.2 Framed protocol (optional)

For full protocol compliance (handshake + framed frames), see [plugin-protocol-spec-v0.1.md](plugin-protocol-spec-v0.1.md). The first frame from Crusty is a JSON handshake; then payload frames (4-byte length + payload). Your plugin must read/write that format if you choose this mode.

## 4. Type validation

For the pipeline to validate, **Output(previous stage) ∩ Input(your plugin) ≠ ∅**. Declare `input` and `output` in `plugin.toml` so Crusty can enforce this.

## 5. Reference plugins

- **plugins/sample-tts** — Shell script TTS stub.
- **plugins/mp3-converter** — Shell script converter (pass-through).
- **plugins/example-python** — Minimal Python TTS stub.

Use them as templates; you can implement in any language (Rust, Python, Go, etc.) as long as you respect the env and stdout contract above.
