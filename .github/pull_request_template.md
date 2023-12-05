<!--
if this PR closes one or more issues, you can automatically link the PR with
them by using one of the [*linking keywords*](https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue#linking-a-pull-request-to-an-issue-using-a-keyword), e.g.
- this PR should close #xxxx
- fixes #xxxx

you can also mention related issues, PRs or discussions!
-->

# Description
<!--
Thank you for improving Nushell. Please, check our [contributing guide](../CONTRIBUTING.md) and talk to the core team before making major changes.

Description of your pull request goes here. **Provide examples and/or screenshots** if your changes affect the user experience.
-->

# User-Facing Changes
<!-- List of all changes that impact the user experience here. This helps us keep track of breaking changes. -->

# Tests + Formatting
<!--
Don't forget to add tests that cover your changes.

Make sure you've run and fixed any issues with these commands:

- `cargo fmt --all -- --check` to check standard code formatting (`cargo fmt --all` applies these changes)
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` to check that you're using the standard code style
- `cargo test --workspace` to check that all tests pass (on Windows make sure to [enable developer mode](https://learn.microsoft.com/en-us/windows/apps/get-started/developer-mode-features-and-debugging))
- `cargo run -- -c "use std testing; testing run-tests --path crates/nu-std"` to run the tests for the standard library

> **Note**
> from `nushell` you can also use the `toolkit` as follows
> ```bash
> use toolkit.nu  # or use an `env_change` hook to activate it automatically
> toolkit check pr
> ```
-->

# After Submitting
<!-- If your PR had any user-facing changes, update [the documentation](https://github.com/nushell/nushell.github.io) after the PR is merged, if necessary. This will help us keep the docs up to date. -->
