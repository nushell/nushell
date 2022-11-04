# Contributing

Welcome to Nushell and thank you for considering contributing!

First of all, before diving into the code, if you plan on making a non-tirival change, such as adding a new feature and especially a breaking change, please make sure to get an approval from the core team.
This saves both your and our time if we realize your change needs to go another direction before the actual coding and reviewing work.
So, please, reach out to us and tell us what you want to do.
This will significantly increase the chance of your PR being accepted.
The best place to do so is on [Discord](https://discordapp.com/invite/NtAbbGn) or [GitHub](https://github.com/nushell/nushell/issues/new/choose).

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
