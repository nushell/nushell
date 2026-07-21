# AGENTS.md

## Testing and code style rules

- Never make any commits or pushes without first requesting if you can.
- When you are complete, ask to run `clippy fmt --all`. Then ask to run clippy on code with `cargo clippy --workspace --exclude "nu_plugin_*" --profile ci --all-targets -- -D warnings -D clippy::unwrap_used -D clippy::unchecked_time_subtraction`. The ask to run clippy on tests with `cargo clippy --workspace --tests --exclude "nu_plugin_*" --profile ci --all-targets -- -D warnings -D clippy::unchecked_time_subtraction`.
- When you are complete you can run tests with `cargo test --workspace --exclude "nu_plugin_*" --all-targets --profile ci`.
- Never use `.unwrap()` except in tests - always handle errors with `ShellError` or `ParseError`
- When running commands with `nu -c ""` never use `echo` like `nu -c "echo blah"`, instead just omit it like `nu -c "blah"`
- When you need to escape characters like `\n` make sure you use double quotes like `"\n"` because in nushell only double quotes interprets escaped characters.
- No panicking on user input, no nightly features, no GPL deps (MIT, Apache License 2.0, CC0 only)
- Commands in `crates/nu-command/src/` implement `Command` trait - add examples in `examples()` (they become tests)
- Dependencies: use workspace dependencies, exact semver `"1.2.3"`, no git dependenciess in PRs
- Wait for any `cargo` command to finish running before starting another one.
- Make sure that all the changes are written in idiomatic Rust with minimal code duplication (DRY). Also ensure that all new changes are well documented for easy maintainability.
- Try to avoid doing `cargo clean` because it just increases compile time the next time you have to compile.
- Make sure you use `CARGO_TARGET_DIR` to find where binaries are built.
- See [rust_style.md](rust_style.md), [FAQ.md](FAQ.md), [HOWTOS.md](HOWTOS.md)

## PR requirements

- If asked to create a pull request, you MUST follow the PR template in `.github/pull_request_template.md` making sure to read the comments in the PR template for a description of what you need to add.
- Title format: `Fix URL parsing in `http get` (#1234)`
- Must pass: `nu -c "use toolkit.nu; toolkit check pr"` (fmt, clippy, test, stdlib) Only run when ready, at the very end, because it takes a while to run. Ask first, do not run this automatically.
- See [CONTRIBUTING.md](../CONTRIBUTING.md)
