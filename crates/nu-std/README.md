<h1 align="center">
  Welcome to the standard library of `nushell`!
  <img src="https://media.giphy.com/media/hvRJCLFzcasrR4ia7z/giphy.gif" width="28"></img>
</h1>

The standard library is a pure-`nushell` collection of commands to allow anyone to build
complex applications using standardized tools gathered incrementally.

In this library, you might find `rust`-like `assert` commands to write tests, tools to
manipulate paths and strings, etc, etc, ...

## :toolbox: use the standard library in the REPL or in scripts
in order to "import" the standard library to either the interactive [*REPL*][REPL] of
`nushell` or inside some `.nu` script, you might want to use the
[`use`](https://nushell.sh/commands/docs/use.html) command!
```bash
use std
```

## :pencil2: contribute to the standard library
- all the commands of the standard_library are located in [`std.nu`](std.nu)
- the tests are located in files that have a name starting with "test_", e.g. [`test_std.nu`](test_std.nu)
- a test runner, at [`tests.nu`](tests.nu), allows to run all the tests automatically

### :wrench: add new commands
- add new standard commands by appending to [`std.nu`](std.nu)
- add associated tests to [`test_std.nu`](tests_std.nu) or preferably to `test_<submodule>.nu`.
    - define a new exported (!) `test_<feature>` command
    - import the `assert` functions you need at the top of the functions, e.g. `use std.nu "assert eq"`

### :test_tube: run the tests
the following call should return no errors
```bash
NU_LOG_LEVEL=DEBUG cargo run -- -c "use std; std testing run --path crates/nu-std"
```

> **Warning**  
> the `cargo run --` part of this command is important to ensure the version of `nushell` and the version of the library are the same.
