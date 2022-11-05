# Description

(Description of your pull request goes here. **Provide examples and/or screenshots** unless the change is trivial)

# Before Submitting

If your change is a new feature, change to an existing feature, or a breaking change, make sure you've got a green light from the core team either on [Discord](https://discordapp.com/invite/NtAbbGn) or [GitHub issue](https://github.com/nushell/nushell/issues/new/choose).
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

* Get another regular contributor to review the PR.
* If your PR affects the user experience of Nushell (adding/removing a command, changing an input/output type, etc.), make sure the changes are reflected in the documentation (https://github.com/nushell/nushell.github.io) after the PR is merged. This helps us keep the docs up to date.
