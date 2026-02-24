# AGENTS.md

## Testing and code style rules

- Before every commit: `nu -c "use toolkit.nu; toolkit fmt"` and `nu -c "use toolkit.nu; toolkit clippy"`. This is pretty fast so you can run it frequently.
- Run tests: `nu -c "use toolkit.nu; toolkit test"` or `cargo test --workspace`. Tests will take a while and are included in `toolkit check pr` so there's no need to run this if you've already run `toolkit check pr`.
- **Never use `.unwrap()`** - always handle errors with `ShellError` or `ParseError`
- When running commands with `nu -c ""` never use `echo` like `nu -c "echo blah"`, instead just omit it like `nu -c "blah"`
- When you need to escape characters like `\n` make sure you use double quotes like `"\n"` because in nushell only double quotes interprets escaped characters.
- No panicking on user input, no nightly features, no GPL deps (MIT, Apache License 2.0, CC0 only)
- Commands in `crates/nu-command/src/` implement `Command` trait - add examples in `examples()` (they become tests)
- Dependencies: use workspace dependencies, exact semver `"1.2.3"`, no git dependenciess in PRs
- Wait for any `cargo` command to finish running before starting another one.
- Make sure that all the changes are written in idiomatic Rust, remove duplication if possible, and make sure the changes are well documented for easy maintainability.
- Try to avoid doing `cargo clean` because it just increases compile time the next time you have to compile.
- Make sure you use CARGO_TARGET_DIR to find where binaries are built.
- See [rust_style.md](rust_style.md), [FAQ.md](FAQ.md), [HOWTOS.md](HOWTOS.md)

## PR requirements

- Title format: `Fix URL parsing in `http get` (#1234)`
- Must pass: `nu -c "use toolkit.nu; toolkit check pr"` (fmt, clippy, test, stdlib) Only run when ready, at the very end, because it takes a while to run.
- Include release notes summary (brief, user-focused)
- Link issues: "Fixes #1234" or "Closes #5678"
- See [CONTRIBUTING.md](../CONTRIBUTING.md)
