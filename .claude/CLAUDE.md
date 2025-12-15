# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

See @../README.md for project overview and @../CONTRIBUTING.md for development workflow.

## Quick Reference

```bash
# Build and run
cargo build
cargo run

# Full PR check (format + clippy + tests + stdlib)
use toolkit.nu
toolkit check pr

# Individual checks
cargo fmt --all
cargo clippy --workspace --exclude nu_plugin_* -- -D warnings -D clippy::unwrap_used -D clippy::unchecked_duration_subtraction
cargo test --workspace

# Test specific command
cargo test --package nu-cli --test main -- commands::<command_name>
```

## Code Style

- `.unwrap()` is banned - use `.expect("reason")` or proper error handling
- No nightly Rust features
- `unsafe` requires `// SAFETY:` comments
