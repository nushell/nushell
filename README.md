# Engine-q

Engine-q is a project to the current functionality in Nushell. It's still in beta state, and there are still some places that need help: please see [the final checklist](https://github.com/nushell/engine-q/issues/735).

## Contributing

If you'd like to help out, come join us on the [discord](https://discord.gg/NtAbbGn) or propose some work in an issue or PR draft. We're currently looking to begin porting Nushell commands to engine-q.

If you are interested in porting a command from Nushell to engine-q you are welcome to
[comment on this issue 735](https://github.com/nushell/engine-q/issues/735) with the command name you would like to port.

## Giving engine-q a test drive

To try out engine-q you need a recent Rust toolchain consisting of the rust compiler and `cargo` (https://www.rust-lang.org/tools/install).

Switch to a directory where you want to create the directory with engine-q code and clone the repository from github with

```
git clone https://github.com/nushell/engine-q.git
# Switch to the newly created directory `engine-q` containing the current source code
cd engine-q
```

Build and run with:

```
cargo run
```

For full performance build and run in release mode

```
cargo run --release
```

If you also want to have access to all ported plugins including dataframe support you need to enable the `extra` features with:

```
cargo run --features extra
```
