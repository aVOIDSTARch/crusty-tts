# Crusty-TTS Plugin Protocol Specification v0.1

This spec defines the data contract and plugin interface. A compliant plugin can be implemented in any language without reading crusty-core source.

## 1. Data Contract

**Unit of data between plugins:** Framed stream on stdin/stdout.

- **Frame format:** `[4-byte length (little-endian uint32)][payload bytes]`
- **Content-type:** Fixed per plugin execution (one input type, one output type per run). No per-frame type tag.
- **Streaming:** Frames are transport chunks, not semantic units. Plugins must not rely on frame boundaries for logic.

## 2. Handshake

The first frame from crusty to the plugin is a **control frame** (JSON, UTF-8).

- **Length:** 4-byte little-endian, then payload.
- **Payload (JSON):**
  - `protocol` (string): e.g. `"0.1"`
  - `input_type` (string): selected input MIME-style type for this run
  - `output_type` (string): selected output type for this run
  - `config` (object): plugin options

After the handshake, all subsequent frames are raw payload of the agreed input type (no JSON wrapper).

## 3. Config Channel

- First frame = handshake with selected types + config.
- Then only payload frames (same format: 4-byte length + payload).

## 4. Error Semantics

- **Structured error:** Plugin may emit a JSON frame with `type`, `message`, `fatal` before exiting.
- **Exit:** On error, plugin exits non-zero.
- **Core behavior:** If plugin exits non-zero without a prior error frame, treat as `INTERNAL_PLUGIN_FAILURE`. If `fatal: true`, abort pipeline.

## 5. Versioning

- **Manifest:** `protocol_version` (or `api_version`) in plugin.toml.
- **Core:** Refuses incompatible versions; no silent fallback.

## 6. Plugin Manifest Schema (plugin.toml)

- **Required:** `name`, `version`, `protocol_version` (or `api_version`), `entrypoint` (path to binary/script, or implied by directory).
- **Capabilities:** `input` (list of MIME-style types), `output` (list), `mode` ("streaming" | "batch").
- **Options schema:** e.g. `[options.voice] type = "string"`, `[options.rate] type = "number" default = 1.0`.
- **Rules:** Statically parseable; no runtime capability mutation in v1.

## 7. Orchestration Validation Rule

For linear pipeline A → B → C:

- `Output(A) ∩ Input(B) ≠ ∅`
- `Output(B) ∩ Input(C) ≠ ∅`

No implicit conversions or auto-inserted adapters in v1.

## 8. Decisions Log

- Content-type per execution (not per frame).
- Config in first frame (handshake).
- Frames = transport slices; no semantic boundaries.
