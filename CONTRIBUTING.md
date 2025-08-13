# Contributing

Welcome to Nushell and thank you for considering contributing!

## Table of contents
- [Tips for submitting PRs](#tips-for-submitting-prs)
- [Proposing design changes](#proposing-design-changes)
- [Developing](#developing)
  - [Setup](#setup)
  - [Tests](#tests)
  - [Useful commands](#useful-commands)
  - [Debugging tips](#debugging-tips)
- [Git etiquette](#git-etiquette)
- [License](#license)

## Other helpful resources

More resources can be found in the nascent [developer documentation](devdocs/README.md) in this repo.

- [Developer FAQ](devdocs/FAQ.md)
- [Platform support policy](devdocs/PLATFORM_SUPPORT.md)
- [Our Rust style](devdocs/rust_style.md)

## Tips for submitting PRs

Thank you for improving Nushell! We are always glad to see contributions, and we are absolutely willing to talk through the design or implementation of your PR. Come talk with us in [Discord](https://discordapp.com/invite/NtAbbGn), or create a GitHub discussion or draft PR and we can help you work out the details from there.

**Please talk to the core team before making major changes!** See the [proposing design changes](#proposing-design-changes) for more details.

### Release notes section

In our PR template, we have a "Release notes summary" section which will be included in our release notes for our blog.

This section should include all information about your change which is relevant to a user of Nushell. You should try to keep it **brief and simple to understand**, and focus on the ways your change directly impacts the user experience. We highly encourage adding examples and, when relevant, screenshots in this section.

Please make sure to consider both the *intended changes*, such as additions or deliberate breaking changes **and** possible *side effects* that might change how users interact with a command or feature. It's important to think carefully about the ways that your PR might affect any aspect of the user experience, and to document these changes even if they seem minor or aren't directly related to the main purpose of the PR.

This section might not be relevant for all PRs. If your PR is a work in progress, feel free to write "WIP"/"TODO"/etc in this section. You can also write "N/A" if this is a technical change which doesn't impact the user experience.

If you're not sure what to put here, or need some help, **a core team member would be glad to help you out**. We may also makes some tweaks to your release notes section. Please don't take it personally, we just want to make sure our release notes are polished and easy to understand. Once the release notes section is ready, we'll add the (TODO label name) label to indicate that the release notes section is ready to be included in the actual release notes.

### Tests and formatting checks

Our CI system automatically checks formatting and runs our tests. If you're running into an issue, or just want to make sure everything is ready to go before creating your PR, you can run the checks yourself:

```nushell
use toolkit.nu # or use an `env_change` hook to activate it automatically
toolkit check pr
```

Furthermore, you can also runs these checks individually with the subcommands of `toolkit`, or run the underlying commands yourself:

- `cargo fmt --all -- --check` to check standard code formatting (`cargo fmt --all` applies these changes)
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` to check that you're using the standard code style
- `cargo test --workspace` to check that all tests pass (on Windows make sure to enable [developer mode](https://learn.microsoft.com/en-us/windows/apps/get-started/developer-mode-features-and-debugging))
- `cargo run -- -c "use toolkit.nu; toolkit test stdlib"` to run the tests for the standard library

If the checks are passing on your local system, but CI just won't pass, feel free to ask for help from the core team.

### Linking and mentioning issues

If your PR closes one or more issues, you can automatically link the PR with them by using one of the [linking keywords](https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue#linking-a-pull-request-to-an-issue-using-a-keyword):

- This PR should close #xxxx
- Fixes #xxxx

You can also mention related issues, PRs or discussions!

## Proposing design changes

First of all, before diving into the code, if you want to create a new feature, change something significantly, and especially if the change is user-facing, it is a good practice to first get an approval from the core team before starting to work on it.
This saves both your and our time if we realize the change needs to go another direction before spending time on it.
So, please, reach out and tell us what you want to do.
This will significantly increase the chance of your PR being accepted.

The review process can be summarized as follows:
1. You want to make some change to Nushell that is more involved than simple bug-fixing.
2. Go to [Discord](https://discordapp.com/invite/NtAbbGn) or a [GitHub issue](https://github.com/nushell/nushell/issues/new/choose) and chat with some core team members and/or other contributors about it.
3. After getting a green light from the core team, implement the feature, open a pull request (PR) and write a concise but comprehensive description of the change.
4. If your PR includes any user-facing features (such as adding a flag to a command), clearly list them in the PR description.
5. Then, core team members and other regular contributors will review the PR and suggest changes.
6. When we all agree, the PR will be merged.
7. If your PR includes any user-facing features, make sure the changes are also reflected in [the documentation](https://github.com/nushell/nushell.github.io) after the PR is merged.
8. Congratulate yourself, you just improved Nushell! :-)

## Developing

### Setup

Nushell requires a recent Rust toolchain and some dependencies; [refer to the Nu Book for up-to-date requirements](https://www.nushell.sh/book/installation.html#build-from-source). After installing dependencies, you should be able to clone+build Nu like any other Rust project:

```bash
git clone https://github.com/nushell/nushell
cd nushell
cargo build
```

### Tests

It is good practice to cover your changes with a test. Also, try to think about corner cases and various ways how your changes could break. Cover those in the tests as well.

Tests can be found in different places:
* `/tests`
* command examples
* crate-specific tests

Most of the tests are built upon the `nu-test-support` crate. For testing specific features, such as running Nushell in a REPL mode, we have so called "testbins". For simple tests, you can find `run_test()` and `fail_test()` functions.

### Useful Commands

As Nushell is built using a cargo workspace consisting of multiple crates keep in mind that you may need to pass additional flags compared to how you may be used to it from a single crate project.
Read cargo's documentation for more details: https://doc.rust-lang.org/cargo/reference/workspaces.html

- Build and run Nushell:

  ```nushell
  cargo run
  ```

- Run Clippy on Nushell:

  ```nushell
  cargo clippy --workspace -- -D warnings -D clippy::unwrap_used
  ```
  or via the `toolkit.nu` command:
  ```nushell
  use toolkit.nu clippy
  clippy
  ```

- Run all tests:

  ```nushell
  cargo test --workspace
  ```

  or via the `toolkit.nu` command:
  ```nushell
  use toolkit.nu test
  test
  ```

- Run all tests for a specific command

  ```nushell
  cargo test --package nu-cli --test main -- commands::<command_name_here>
  ```

- Check to see if there are code formatting issues

  ```nushell
  cargo fmt --all -- --check
  ```
  or via the `toolkit.nu` command:
  ```nushell
  use toolkit.nu fmt
  fmt --check
  ```

- Format the code in the project

  ```nushell
  cargo fmt --all
  ```
  or via the `toolkit.nu` command:
  ```nushell
  use toolkit.nu fmt
  fmt
  ```

- Set up `git` hooks to check formatting and run `clippy` before committing and pushing:

  ```nushell
  use toolkit.nu setup-git-hooks
  setup-git-hooks
  ```
  _Unfortunately, this hook isn't available on Windows._

### Debugging Tips

- To view verbose logs when developing, enable the `trace` log level.

  ```nushell
  cargo run --release -- --log-level trace
  ```

- To redirect trace logs to a file, enable the `--log-target file` switch.
  ```nushell
  cargo run --release -- --log-level trace --log-target file
  open $"($nu.temp-path)/nu-($nu.pid).log"
  ```

## Git etiquette

As nushell thrives on its broad base of volunteer contributors and maintainers with different backgrounds we have a few guidelines for how we best utilize git and GitHub for our contributions. We strive to balance three goals with those recommendations:

1. The **volunteer maintainers and contributors** can easily follow the changes you propose, gauge the impact, and come to help you or make a decision.
2. **You as a contributor** can focus most of your time on improving the quality of the nushell project and contributing your expertise to the code or documentation.
3. Making sure we can trace back *why* decisions were made in the past.
This includes discarded approaches. Also we want to quickly identify regressions and fix when something broke.

### How we merge PRs

In general the maintainers **squash** all changes of your PR into a single commit when merging.

This keeps a clean enough linear history, while not forcing you to conform to a too strict style while iterating in your PR or fixing small problems. As an added benefit the commits on the `main` branch are tied to the discussion that happened in the PR through their `#1234` issue number.

> **Note**
> **Pro advice:** In some circumstances, we can agree on rebase-merging a particularly large but connected PR as a series of atomic commits onto the `main` branch to ensure we can more easily revert or bisect particular aspects.

### A good PR makes a change!

As a result of this PR-centric strategy and the general goal that the reviewers should easily understand your change, the **PR title and description matters** a great deal!

Make sure your description is **concise** but contains all relevant information and context.
This means demonstrating what changes, ideally through nushell code or output **examples**.
Furthermore links to technical documentation or instructions for folks that want to play with your change make the review process much easier.

> **Note**
> Try to follow the suggestions in our PR message template to make sure we can quickly focus on the technical merits and impact on the users.

#### A PR should limit itself to a single functional change or related set of same changes.

Mixing different changes in the same PR will make the review process much harder. A PR might get stuck on one aspect while we would actually like to land another change. Furthermore, if we are forced to revert a change, mixing and matching different aspects makes fixing bugs or regressions much harder.

Thus, please try to **separate out unrelated changes**!
**Don't** mix unrelated refactors with a potentially contested change.
Stylistic fixes and housekeeping can be bundled up into singular PRs.

#### Guidelines for the PR title

The PR title should be concise but contain everything for a contributor to know if they should help out in the review of this particular change.

**DON'T**
- `Update file/in/some/deeply/nested/path.rs`
  - Why are you making this change?
- `Fix 2134`
  - What has to be fixed?
  - Hard to follow when not online on GitHub.
- ``Ignore `~` expansion``
  - In what context should this change take effect?
- `[feature] refactor the whole parser and also make nushell indentation-sensitive, upgrade to using Cpython. Let me know what you think!`
  - Be concise
  - Maybe break up into smaller commits or PRs if the title already appears too long?

**DO**
- Mention the nushell feature or command that is affected.
  - ``Fix URL parsing in `http get` (issue #1234)``
- You can mention the issue number if other context is there.
  - In general, mention all related issues in the description to crosslink (e.g. `Fixes #1234`, `Closes #6789`)
- For internal changes mention the area or symbols affected if it helps to clarify
  - ``Factor out `quote_string()` from parser to reuse in `explore` ``

### Review process / Merge conflicts

> **Note**
> Keep in mind that the maintainers are volunteers that need to allocate their attention to several different areas and active PRs. We will try to get back to you as soon as possible.

You can help us to make the review process a smooth experience:
- Testing:
  - We generally review in detail after all the tests pass. Let us know if there is a problem you want to discuss to fix a test failure or forces us to accept a breaking change.
  - If you fix a bug, it is highly recommended that you add a test that reproduces the original issue/panic in a minimal form.
  - In general, added tests help us to understand which assumptions go into a particular addition/change.
  - Try to also test corner cases where those assumptions might break. This can be more valuable than simply adding many similar tests.
- Commit history inside a PR during code review:
  - Good **atomic commits** can help follow larger changes, but we are not pedantic.
  - We don't shame fixup commits while you try to figure out a problem. They can help others see what you tried and what didn't work. (see our [squash policy](#how-we-merge-prs))
  - During active review constant **force pushing** just to amend changes can be confusing!
    - GitHub's UI presents reviewers with less options to compare diffs
    - fetched branches for experimentation become invalid!
    - the notification a maintainer receives has a low signal-to-noise ratio
  - Git pros *can* use their judgement to rebase/squash to clean up the history *if it aids the understanding* of a larger change during review
- Merge conflicts:
  - In general you should take care of resolving merge conflicts.
    - Use your judgement whether to `git merge main` or to `git rebase main`
    - Choose what simplifies having confidence in the conflict resolution and the review. **Merge commits in your branch are OK** in the squash model.
  - Feel free to notify your reviewers or affected PR authors if your change might cause larger conflicts with another change.
  - During the rollup of multiple PRs, we may choose to resolve merge conflicts and CI failures ourselves. (Allow maintainers to push to your branch to enable us to do this quickly.)

## License

We use the [MIT License](https://github.com/nushell/nushell/blob/main/LICENSE) in all of our Nushell projects. If you are including or referencing a crate that uses the [GPL License](https://www.gnu.org/licenses/gpl-3.0.en.html#license-text) unfortunately we will not be able to accept your PR.
