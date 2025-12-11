# AGENTS.md

## Setup and build

- Clone and build: `git clone https://github.com/nushell/nushell && cd nushell && cargo build`
- Run from source: `cargo run` or `use toolkit.nu; toolkit run`
- Use `toolkit.nu` for all dev tasks: `use toolkit.nu`
- Key crates in `crates/`: `nu-command` (built-ins), `nu-parser`, `nu-protocol` (types/errors), `nu-engine`, `nu-cli`
- Rust version in `rust-toolchain.toml` (typically 2 releases behind stable)

## Testing and code style

- Before every commit: `toolkit fmt && toolkit clippy`
- Before every PR: `toolkit check pr` (runs fmt, clippy, test, test stdlib)
- Run tests: `toolkit test` or `cargo test --workspace`
- **Never use `.unwrap()`** - always handle errors with `ShellError` or `ParseError`
- No panicking on user input, no nightly features, no GPL deps (MIT only)
- Commands in `crates/nu-command/src/` implement `Command` trait - add examples in `examples()` (they become tests)
- Check deps: `cargo tree --duplicates` - use workspace deps, exact semver `"1.2.3"`, no git deps in PRs
- See [rust_style.md](rust_style.md), [FAQ.md](FAQ.md), [HOWTOS.md](HOWTOS.md)

## PR requirements

- Title format: ``Fix URL parsing in `http get` (#1234)``
- Must pass: `toolkit check pr` (fmt, clippy, test, stdlib)
- Include release notes summary (brief, user-focused)
- Link issues: "Fixes #1234" or "Closes #5678"
- Major changes: discuss on [Discord](https://discordapp.com/invite/NtAbbGn) first
- See [CONTRIBUTING.md](../CONTRIBUTING.md)
