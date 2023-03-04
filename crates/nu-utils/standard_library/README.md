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
use /path/to/standard_library/std.nu
```

### :mag: a concrete example
- my name is @amtoine and i use the `ghq` tool to manage `git` projects
> **Note**  
> `ghq` stores any repository inside `$env.GHQ_ROOT` under `<host>/<owner>/<repo>/`
- the path to my local fork of `nushell` is then defined as
```bash
let-env NUSHELL_REPO = ($env.GHQ_ROOT | path join "github.com" "amtoine" "nushell")
```
- and the full path to the standard library is defined as
```bash
let-env STD_LIB = ($env.NUSHELL_REPO | path join "crates" "nu-utils" "standard_library")
```
> see the content of `$env.STD_LIB` :yum:
> ```bash
> >_ ls $env.STD_LIB | get name | str replace $env.STD_LIB "" | str trim -l -c "/"
> ╭───┬───────────╮
> │ 0 │ README.md │
> │ 1 │ std.nu    │
> │ 2 │ tests.nu  │
> ╰───┴───────────╯
> ```
- finally we can `use` the standard library and have access to the commands it exposes :thumbsup:
```bash
>_ use std.nu
>_ help std
Module: std

Exported commands:
  assert (std assert), assert eq (std assert eq), assert ne (std assert ne), match (std match)

This module does not export environment.
```

## :pencil2: contribute to the standard library
### :wrench: add new commands
- add new standard commands to [`std.nu`](std.nu)
- add associated tests to [`tests.nu`](tests.nu)
    - define a new `test_<feature>` before the `main`
    - import the `assert` functions you need at the top of the functions, e.g. `use std.nu "assert eq"`
    - add a call to `test_<feature>` at the bottom of the `main`

### :test_tube: run the tests
the following call should return nothing
```bash
nu ($env.STD_LIB | path join "tests.nu")
```

[REPL]: https://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop
