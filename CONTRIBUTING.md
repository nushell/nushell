Welcome to nushell!

*Note: for a more complete guide see [The nu contributor book](https://github.com/nushell/contributor-book)*

For speedy contributions open it in Gitpod, nu will be pre-installed with the latest build in a VSCode like editor all from your browser.

[![Open in Gitpod](https://gitpod.io/button/open-in-gitpod.svg)](https://gitpod.io/#https://github.com/nushell/nushell)

To get live support from the community see our [Discord](https://discordapp.com/invite/NtAbbGn), [Twitter](https://twitter.com/nu_shell) or file an issue or feature request here on [GitHub](https://github.com/nushell/nushell/issues/new/choose)!
<!--WIP-->

# Developing
## Set up
This is no different than other Rust projects.

```shell
git clone https://github.com/nushell/nushell
cd nushell
cargo build
```

## Useful Commands

Build and run Nushell:

```shell
cargo build --release && cargo run --release
```

Run Clippy on Nushell:

```shell
cargo clippy --all --features=stable
```

Run all tests:

```shell
cargo test --all --features=stable
```

Run all tests for a specific command

```shell
cargo test --package nu-cli --test main -- commands::<command_name_here>
```

Check to see if there are code formatting issues

```shell
cargo fmt --all -- --check
```

Format the code in the project

```shell
cargo fmt --all
```
