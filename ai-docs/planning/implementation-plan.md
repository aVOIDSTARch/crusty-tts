# Crusty-TTS Implementation Plan

This document drafts a phased plan to implement the full vision described in `original-chat-formatted.md`. Crusty-TTS is a **headless, modular audio synthesis runtime**: plugin-driven orchestration with subprocess plugins, declarative pipelines, and a dual-mode runtime (CLI + daemon) exposing an API for any interface (CLI, web, app).

---

## 1. Vision Summary

| Concept | Description |
|--------|-------------|
| **Core** | Orchestrator only: registry, validator, pipeline executor, capability broker. No TTS/audio logic inside core. |
| **Plugins** | Subprocess workers; protocol over stdin/stdout. Language-agnostic (Rust, Python, Go, etc.). |
| **Pipeline** | Linear DAG: Pre → Synth → Encode (e.g. MP3) → Post → Output. v1: linear only, no branching. |
| **Orchestration** | `.cr` (or `.or`) file: TOML pipeline definition, reusable and API-editable. |
| **Interfaces** | Core exposes API only. CLI and daemon are thin shells; UIs (web, app, CLI wizard) are built by developers against the API. |
| **Security** | Verify manifests, run health/mini-test, optional checksum/signature for URL-sourced plugins. |

---

## 2. Phased Implementation Overview

| Phase | Focus | Outcome |
|-------|--------|--------|
| **0** | Protocol & docs | Frozen plugin protocol spec; data contract, handshake, framing, errors. |
| **1** | Core library (crusty-core) | Manifest parsing, plugin discovery, verification, pipeline validation, stream execution. |
| **2** | CLI (crusty-cli) | `crusty run orchestration.cr input.txt --output out.mp3`; same execution as daemon. |
| **3** | Reference plugins | At least one Rust and one non-Rust (e.g. Python) plugin implementing the protocol. |
| **4** | Daemon & API | Long-lived process; REST (or gRPC) API; job management, streaming. |
| **5** | Config builder & API for UIs | Interactive CLI wizard and/or API to list plugins, build orchestration, save .cr. |
| **6** | Optional: URL plugins, hardening | Fetch plugins by URL with verification; daemon config (ports, sandbox, limits). |

---

## 3. Phase 0: Protocol First (Edges)

**Principle:** Define the data contract and plugin interface before building the engine. No implementation of core execution until the protocol is written and stable.

### 3.1 Deliverables

1. **Plugin Protocol Specification (standalone doc)**
   - **Data contract:** What is the unit of data between plugins?
     - Chosen: framed stream on stdin/stdout.
     - Frame format: `[4-byte length (little-endian uint32)][payload bytes]`.
     - Content-type: **fixed per plugin execution** (one input type, one output type per run).
   - **Handshake:** First frame is a control frame (JSON, UTF-8). Contains:
     - `protocol`, `input_type`, `output_type`, `config` (plugin options).
   - **Config channel:** First frame = handshake with selected types + config; then only payload frames.
   - **Streaming:** Frames are transport chunks, not semantic units; plugins must not rely on frame boundaries for logic.
   - **Error semantics:** Structured error frame (e.g. JSON with `type`, `message`, `fatal`); then exit non-zero. If exit non-zero without error frame → treat as `INTERNAL_PLUGIN_FAILURE`.
   - **Versioning:** `protocol_version` in manifest; core refuses incompatible versions.

2. **Plugin Manifest Schema (plugin.toml)**
   - Required: `name`, `version`, `protocol_version`, `entrypoint` (path to binary/script).
   - Capabilities: `input` (list of MIME-style types), `output` (list), `mode` ("streaming" | "batch").
   - Options schema: e.g. `[options.voice] type = "string"`, `[options.rate] type = "number" default = 1.0`.
   - Rules: statically parseable; no runtime capability mutation in v1.

3. **Orchestration Validation Rule**
   - For linear pipeline A → B → C: `Output(A) ∩ Input(B) ≠ ∅`, `Output(B) ∩ Input(C) ≠ ∅`.
   - No implicit conversions or auto-inserted adapters in v1.

4. **Decisions log**
   - Content-type per execution (not per frame).
   - Config in first frame (handshake).
   - Frames = transport slices; no semantic boundaries.

### 3.2 Outputs

- `docs/plugin-protocol-spec-v0.1.md` (or similar): full spec a stranger could implement from.
- Manifest schema defined (e.g. in spec or separate schema file).
- Optional: minimal sequence diagram for spawn → handshake → stream → end.

---

## 4. Phase 1: Core Library (crusty-core)

**Principle:** One library crate that owns all orchestration logic. No business logic in CLI or daemon.

### 4.1 Crate layout

```
crusty-tts/
  crates/
    crusty-core/     # library
    crusty-cli/      # binary (Phase 2)
    crusty-daemon/   # binary (Phase 4)
  plugins/           # local plugin directories (each with plugin.toml + entrypoint)
  docs/
```

### 4.2 Core responsibilities

| Component | Responsibility |
|-----------|----------------|
| **Manifest parsing** | Parse plugin.toml; validate against schema; expose structured PluginInfo (name, version, capabilities, options schema). |
| **Plugin discovery** | Scan `plugins/` (and optionally configured dirs); aggregate manifests. |
| **Verification** | Per plugin: spawn subprocess, send handshake, run minimal test (e.g. small synth or identity), check output type; fail fast on mismatch or crash. |
| **Capability index** | Stage → plugins; InputType → plugins; OutputType → plugins. Used for validation and for UI/API. |
| **Orchestration parser** | Parse .cr (TOML); structure: nodes (id, plugin, options), flow order. |
| **Orchestration validator** | Check output type of node N matches input type of node N+1; check plugin exists and is verified; no branching in v1. |
| **Pipeline executor** | Single function, e.g. `execute_pipeline(orchestration, input_stream) -> Result<OutputStream>`. Spawns plugins in order, sends handshake, streams frames, propagates errors. Same code path for CLI and daemon. |
| **Streaming** | Internal representation is stream-based (e.g. `Stream<Item = Vec<u8>>` or equivalent); no “load whole book into memory” by default. |

### 4.3 Dependencies (suggested)

- `toml`, `serde` for manifest and orchestration.
- `tokio` (or async runtime) for async streaming if needed.
- `anyhow` / `thiserror` for errors.
- Std process: `std::process::Command` for subprocess spawn; stdin/stdout pipes for framed I/O.

### 4.4 Out of scope for core

- No SSML parsing inside core.
- No audio decoding/encoding inside core.
- No HTTP/gRPC (lives in crusty-daemon).

---

## 5. Phase 2: CLI (crusty-cli)

- **Thin binary** that depends on crusty-core.
- **Usage (target):** `crusty run orchestration.cr input.txt --output out.mp3` (or `--output -` for stdout).
- **Behavior:** Load orchestration, (optionally) verify plugins, run `execute_pipeline(orchestration, input_stream)`, stream result to file or stdout, exit.
- **Stateless:** No daemon state; no job store. Same execution function as daemon.

---

## 6. Phase 3: Reference Plugins

- **Rust plugin:** Minimal plugin that:
  - Reads handshake (first frame).
  - Accepts payload frames on stdin; writes output frames to stdout.
  - Supports at least one input type and one output type (e.g. text/plain → audio/wav stub or real minimal TTS).
  - Emits structured error on invalid input.
- **Python (or other) plugin:** Same contract; proves protocol is language-agnostic.
- **Goal:** Both must be implementable without reading crusty-core source. If not, refine protocol.

Optional: one “pre” (e.g. normalize text), one “post” (e.g. pass-through or simple effect), one encoder (e.g. WAV→MP3 via external binary) to validate full chain.

---

## 7. Phase 4: Daemon & API

- **crusty-daemon:** Long-lived process. Loads plugin registry at startup; exposes API.
- **Same execution:** Daemon calls the same `execute_pipeline` from crusty-core; no duplicate logic.
- **Job management:** Job ID, status (pending/running/done/failed), cancellation, streaming endpoint.
- **API (REST suggested for v1):**
  - `GET /plugins` — list installed plugins.
  - `GET /plugins/{id}` — get plugin capabilities/options schema.
  - `POST /pipeline/validate` — validate a pipeline (orchestration payload).
  - `POST /pipeline/run` — submit job; return job ID.
  - `GET /jobs/{id}/status` — status.
  - `GET /jobs/{id}/stream` — stream output (or equivalent).
- **Config:** Optional `daemon.toml`: ports, plugin dirs, timeouts, resource limits (future).

---

## 8. Phase 5: Config Builder & API for UIs

- **Interactive CLI wizard:** Discover plugins → select active plugins → set options → save orchestration.cr. (Can be same binary as crusty-cli, e.g. `crusty configure`.)
- **API surface for UIs:** Already partly covered in Phase 4 (list plugins, capabilities, validate, run). Add/surface:
  - Return option schemas so UIs can build forms dynamically.
  - Accept orchestration as JSON (or TOML) and save as .cr.
- **No GUI in repo:** GUI is a separate project that consumes the API.

---

## 9. Phase 6: Optional Hardening & URL Plugins

- **URL-sourced plugins:** Download from URL; verify checksum (e.g. SHA256) and optionally signature (e.g. ed25519); extract to sandbox dir; then treat as local plugin (manifest + verification).
- **daemon.toml:** Ports, timeouts, sandbox options, resource limits.
- **Security:** Document subprocess isolation; optional seccomp/separate user for plugins.

---

## 10. v1 Constraints (Non-Negotiable for First Release)

- **Linear pipelines only.** No branching, no parallel nodes, no distributed execution.
- **Content-type:** One input and one output type per plugin execution; agreed at handshake.
- **No implicit type coercion** between nodes; validation must enforce type compatibility.
- **Plugins unaware of mode** (CLI vs daemon).
- **Core stays dumb** about audio/text internals: route, validate, orchestrate, supervise only.

---

## 11. Suggested File Layout (End State)

```
crusty-tts/
  Cargo.toml              # workspace
  crates/
    crusty-core/
      Cargo.toml
      src/
        lib.rs
        manifest.rs       # plugin.toml parsing & schema
        discovery.rs      # scan plugins dir
        verify.rs         # health check + mini-test
        capability.rs     # capability index
        orchestration.rs  # .cr parse + validate
        pipeline.rs       # execute_pipeline, stream wiring
        protocol.rs       # frame format, handshake, errors
    crusty-cli/
      Cargo.toml
      src/main.rs         # run, configure (wizard)
    crusty-daemon/
      Cargo.toml
      src/main.rs         # HTTP server, job store, call core
  plugins/
    example-rust/         # reference Rust plugin
    example-python/       # reference Python plugin
  docs/
    plugin-protocol-spec-v0.1.md
  orchestration.cr        # example
  daemon.toml             # optional
```

---

## 12. Order of Implementation (Checklist)

1. **Phase 0**
   - [x] Write plugin protocol spec (framing, handshake, errors, versioning). → `docs/plugin-protocol-spec-v0.1.md`
   - [x] Define plugin.toml schema and validation rules. → In spec + crusty-core manifest parsing.
   - [x] Document orchestration validation rule (type intersection). → In spec + `validate.rs`.

2. **Phase 1**
   - [x] Create workspace and crusty-core crate.
   - [x] Implement manifest parsing and schema validation.
   - [x] Implement plugin discovery (plugins/).
   - [x] Implement verification (spawn, handshake, mini-test).
   - [x] Build capability index from manifests.
   - [x] Implement orchestration .cr parser and validator.
   - [x] Implement pipeline executor (stream-based, subprocess per node).
   - [x] Unit tests for validation and executor with mock plugins.

3. **Phase 2**
   - [x] Create crusty-cli; implement `crusty run`.
   - [x] Wire input file/stream and output file/stdout (including `--output -` for stdout).
   - [x] Integration: run with stub plugins (sample-tts, mp3-converter).

4. **Phase 3**
   - [x] Implement minimal Rust reference plugin (handshake + frames). → sample-tts (env-based), protocol in core.
   - [x] Implement minimal Python reference plugin. → `plugins/example-python/`.
   - [x] Document “how to implement a plugin” from spec only. → `docs/how-to-implement-a-plugin.md`.
   - [x] Optional: add one pre, one post, one encoder example. → sample-tts, mp3-converter.

5. **Phase 4**
   - [x] Create crusty-daemon binary.
   - [x] Expose REST (or gRPC) endpoints above.
   - [x] Job store (in-memory or simple persistence).
   - [x] Stream endpoint for running job output.
   - [x] Use same execute_pipeline from core.

6. **Phase 5**
   - [x] Interactive CLI wizard: discover → select → options → save .cr. → `crusty configure`.
   - [x] API: option schemas for dynamic UI; accept/save orchestration. → GET /plugins/{id} returns options; POST /pipeline/run accepts orchestration.

7. **Phase 6 (optional)**
   - [ ] URL plugin fetch + verification (checksum/signature).
   - [ ] daemon.toml and security notes.

---

## 13. Success Criteria

- **Protocol:** A developer can implement a compliant plugin in another language without reading crusty-core source.
- **Core:** Single execution path for pipeline; CLI and daemon both use it.
- **Orchestration:** Validation fails fast on type mismatch; no silent coercion.
- **Interfaces:** Any client (CLI, web, app) can list plugins, build a valid pipeline, and run it via the same API/execution.

This plan aligns with the vision in `original-chat-formatted.md` and with the existing `architecture-planning.md` and supplement docs. Start at Phase 0 (protocol and manifest schema); then implement core, CLI, reference plugins, daemon, and config builder in order.
