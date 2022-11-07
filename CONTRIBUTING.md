# Contributing

Welcome to Nushell and thank you for considering contributing!

## Review Process

First of all, before diving into the code, if you want to create a new feature, change something significantly, and especially if the change is user-facing, it is a good practice to first get an approval from the core team before starting to work on it.
This saves both your and our time if we realize the change needs to go another direction before spending time on it.
So, please, reach out and tell us what you want to do.
This will significantly increase the chance of your PR being accepted.

The review process can be summarized as follows:
1. You want to make some change to Nushell that is more involved than simple bug-fixing.
2. Go to [Discord](https://discordapp.com/invite/NtAbbGn) or a [GitHub issue](https://github.com/nushell/nushell/issues/new/choose) and chat with some core team members and/or other contributors about it.
3. After getting a green light from the core team, implement the feature, open a pull request (PR) and write a concise but comprehensive description of the change.
4. If your PR includes any use-facing features (such as adding a flag to a command), clearly list them in the PR description.
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

### Useful Commands

- Build and run Nushell:

  ```shell
  cargo run
  ```

- Build and run with extra features. Currently extra features include dataframes and sqlite database support.
  ```shell
  cargo run --features=extra
  ```

- Run Clippy on Nushell:

  ```shell
  cargo clippy --workspace --features=extra -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect
  ```

- Run all tests:

  ```shell
  cargo test --workspace --features=extra
  ```

- Run all tests for a specific command

  ```shell
  cargo test --package nu-cli --test main -- commands::<command_name_here>
  ```

- Check to see if there are code formatting issues

  ```shell
  cargo fmt --all -- --check
  ```

- Format the code in the project

  ```shell
  cargo fmt --all
  ```

### Debugging Tips

- To view verbose logs when developing, enable the `trace` log level.

  ```shell
  cargo run --release --features=extra -- --log-level trace
  ```

- To redirect trace logs to a file, enable the `--log-target file` switch.
  ```shell
  cargo run --release --features=extra -- --log-level trace --log-target file
  open $"($nu.temp-path)/nu-($nu.pid).log"
  ```
