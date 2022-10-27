# Description

(Description of your pull request goes here. **Provide examples and/or screenshots** unless the change is trivial)

# Tests + Formatting

Make sure you've done the following, if applicable:

- [ ] Add tests that cover your changes (either in the command examples, the crate/tests folder, or in the /tests folder)
  - Try to think about corner cases and various ways how your changes could break. Cover those in the tests

Make sure you've run and fixed any issues with these commands:

- [ ] `cargo fmt --all -- --check` to check standard code formatting (`cargo fmt --all` applies these changes)
- [ ] `cargo clippy --workspace --features=extra -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect` to check that you're using the standard code style
- [ ] `cargo test --workspace --features=extra` to check that all tests pass

# User-Facing Changes

If you're making changes that will affect the user experience of Nushell (ex: adding/removing a command, changing an input/output type, adding a new flag):

- Get another regular contributor to review the PR before merging
- Make sure that there is an entry in the documentation (https://github.com/nushell/nushell.github.io) for the feature, and update it if necessary
