# Plugin protocol JSON Schema

`plugin_protocol.schema.json` is a **documentation-oriented** schema of the plugin protocol envelope.

## Important limits

- It is **not** a complete wire contract.
- Many nested engine payloads (`ShellError`, `Config`, AST fragments, IR, signatures details beyond structure, etc.) are modeled as free-form JSON.
- **Stability is enforced by** `protocol_snapshots/` and unit tests that serialize real Rust types, **not** by this schema.

## Regenerating

```text
cargo run -p nu-plugin-protocol --example generate_protocol_schema --features schema
```

Regenerate when intentionally changing the top-level protocol envelope shape, and keep it in sync with the `schema` module tests (`cargo test -p nu-plugin-protocol --features schema`).
