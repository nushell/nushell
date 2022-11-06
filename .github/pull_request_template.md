# Description

(Description of your pull request goes here. **Provide examples and/or screenshots** if your changes affect the user experience.)

# Major Changes

If you're considering making any major change to nushell, before starting work on it, seek feedback from regular contributors and get approval for the idea from the core team either on [Discord](https://discordapp.com/invite/NtAbbGn) or [GitHub issue](https://github.com/nushell/nushell/issues/new/choose).
Making sure we're all on board with the change saves everybody's time.
Thanks!

# Tests + Formatting

Make sure you've done the following, if applicable:

- Add tests that cover your changes (either in the command examples, the crate/tests folder, or in the /tests folder)
  - Try to think about corner cases and various ways how your changes could break. Cover those in the tests

Make sure you've run and fixed any issues with these commands:

- `cargo fmt --all -- --check` to check standard code formatting (`cargo fmt --all` applies these changes)
- `cargo clippy --workspace --features=extra -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect` to check that you're using the standard code style
- `cargo test --workspace --features=extra` to check that all tests pass

# After Submitting

* Help us keep the docs up to date: If your PR affects the user experience of Nushell (adding/removing a command, changing an input/output type, etc.), make sure the changes are reflected in the documentation (https://github.com/nushell/nushell.github.io) after the PR is merged.
