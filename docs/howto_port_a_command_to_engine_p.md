# How To Port Old Engine Syntax to the Latest and Greatest

## engine-p

Even in the new codebase of nu things change and we have some old code to clean up.
This guide walks you through how to port old engine syntax to engine-p.

1. change into the commands source directory

```sh
cd crates/nu-command/src/
```

2. search for old syntax using ripgrep (`rg`)

```sh
rg --type rust --files-with-matches 'ActionStream|run_with_actions'
```

3. update the old syntax engine-p syntax
  - For a smaller example PR see [#3794](https://github.com/nushell/nushell/pull/3794/files)
  - For a more involved example PR see [#3649](https://github.com/nushell/nushell/pull/3649/files)

In many cases this is changing the function name and signature.
The function name goes from `run_with_actions` to `run`.
The signature goes from `Result<ActionStream, ShellError>`
to `Result<OutputStream, ShellError>`.

Sometimes you cannot alter the signature because the command
still requires working with `ActionStream`.
[open](https://github.com/nushell/nushell/blob/28db8022fef7c1f7f44f6e50d07d3f42773deddf/crates/nu-command/src/commands/filesystem/open.rs) is one example.

4. commit changes, run the tests, filtered by the command package and section name:

```sh
cargo test --features=extra -p nu-command string
```

5. PR and bask in the glory of the nu-and-improved codebase.
