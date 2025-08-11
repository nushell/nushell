<!--
Thank you for improving Nushell!

**Please, read our contributing guide [1] and talk to the core team before making major changes.**

If this PR closes one or more issues, you can automatically link the PR with them by using one of the linking keywords [2], e.g.:

- this PR should close #xxxx
- fixes #xxxx

You can also mention related issues, PRs or discussions!

[1]: https://github.com/nushell/nushell/blob/main/CONTRIBUTING.md
[2]: https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue#linking-a-pull-request-to-an-issue-using-a-keyword
-->

# Release notes summary
<!--
This section will be included as part of our release notes.
Please write a brief summary of your change. We encourage adding examples and screenshots in this section.

If you're not confident about this, a core team member would be glad to help!

If this is a work in progress PR, feel free to write "WIP"/"TODO"/etc.
You can also write "N/A" if this is a technical change which doesn't impact the user experience.
-->

# Additional details
<!-- Provide any additional details, technical or otherwise, which you'd like to note but aren't relevant for the release notes.  -->

# Tests + Formatting
<!--
Don't forget to add tests that cover your changes.

Make sure you've run and fixed any issues with these commands:

- `cargo fmt --all -- --check` to check standard code formatting (`cargo fmt --all` applies these changes)
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` to check that you're using the standard code style
- `cargo test --workspace` to check that all tests pass (on Windows make sure to enable developer mode [1])
- `cargo run -- -c "use toolkit.nu; toolkit test stdlib"` to run the tests for the standard library

From Nushell, you can also use the `toolkit` as follows
> use toolkit.nu  # or use an `env_change` hook to activate it automatically
> toolkit check

[1]: https://learn.microsoft.com/en-us/windows/apps/get-started/developer-mode-features-and-debugging
-->

# After Submitting
<!--
If your PR had any user-facing changes, update the documentation [1] after the PR is merged, if necessary. This will help us keep the docs up to date.

[1]: https://github.com/nushell/nushell.github.io
-->
