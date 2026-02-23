# Crusty-TTS

## Project Information

- Project Author(s) [authors]
- Creation Date [created-on]
- Purpose of Project [purpose]
- Primary Coding Language [lang]
- Github Repository URL [repo-url]

## Testing

Run the full test suite:

```bash
cargo test --workspace
```

- **crusty-core**: 21 unit tests (orchestration, protocol, plugin, registry, validate, pipeline, plugin_runner) + 2 integration tests (execute_pipeline with stub plugin, registry + validate roundtrip).
- **crusty-cli**: 2 integration tests (CLI exit behavior, run with temp orchestration and stub plugin).
- **crusty-daemon**: 6 unit tests (GET /plugins, GET /plugins/:id 404, validate invalid TOML, validate minimal, job status/stream 404).

Unix-only tests (plugin runner echo, CLI run with stub) are gated with `#[cfg(unix)]`.

