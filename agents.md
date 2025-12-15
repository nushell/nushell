# AGENTS.md

## Testing and code style

- Before every commit: `nu -c "use toolkit.nu; toolkit fmt"` and `nu -c "use toolkit.nu; toolkit clippy"`
- Before every PR: `nu -c "use toolkit.nu; toolkit check pr"` (runs fmt, clippy, test, test stdlib)
- Run tests: `nu -c "use toolkit.nu; toolkit test"` or `cargo test --workspace`
- **Never use `.unwrap()`** - always handle errors with `ShellError` or `ParseError`
- No panicking on user input, no nightly features, no GPL deps (MIT only)
- Commands in `crates/nu-command/src/` implement `Command` trait - add examples in `examples()` (they become tests)
- Check deps: `cargo tree --duplicates` - use workspace deps, exact semver `"1.2.3"`, no git deps in PRs
- See [rust_style.md](rust_style.md), [FAQ.md](FAQ.md), [HOWTOS.md](HOWTOS.md)

## PR requirements

- Title format: `Fix URL parsing in `http get` (#1234)`
- Must pass: `nu -c "use toolkit.nu; toolkit check pr"` (fmt, clippy, test, stdlib)
- Include release notes summary (brief, user-focused)
- Link issues: "Fixes #1234" or "Closes #5678"
- See [CONTRIBUTING.md](../CONTRIBUTING.md)
