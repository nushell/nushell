<h1 align="center">
  Welcome to the standard library of `nushell`!
  <img src="https://media.giphy.com/media/hvRJCLFzcasrR4ia7z/giphy.gif" width="28"></img>
</h1>

The standard library is a pure-`nushell` collection of custom commands which
provide interactive utilities and building blocks for users writing casual scripts or complex applications.

To see what's here:
```
> use std
> help commands | select name usage | where name =~ "std "
╭────┬─────────────────────────────┬────────────────────────────────────────────────────────────────╮
│  # │            name             │                                usage                           │
├────┼─────────────────────────────┼────────────────────────────────────────────────────────────────┤
│  0 │ std assert                  │ Universal assert command                                       │
│  1 │ std assert equal            │ Assert $left == $right                                         │
           . . .
│ 11 │ std clip                    │ put the end of a pipe into the system clipboard.               │
│ 12 │ std dirs add                │ Add one or more directories to the list.                       │
           . . .
├────┼─────────────────────────────┼────────────────────────────────────────────────────────────────┤
│  # │            name             │                                usage                           │
╰────┴─────────────────────────────┴────────────────────────────────────────────────────────────────╯
```

## :toolbox: Using the standard library in the REPL or in scripts
All commands in the standard library must be "imported" into the running environment
(the interactive read-execute-print-loop (REPL) or a `.nu` script) using the
[`use`](https://nushell.sh/commands/docs/use.html) command.

You can choose to import the whole module, but then must refer to individual commands with a `std` prefix, e.g:
```nushell
use std

std log debug "Running now"
std assert (1 == 2)
```
Or you can enumerate the specific commands you want to import and invoke them without the `std` prefix.
```nushell
use std ["log debug" assert]

log debug "Running again"
assert (2 == 1)
```
This is probably the form of import you'll want to add to your `env.nu` for interactive use.

## :pencil2: contribute to the standard library
You're invited to contribute to the standard library! See [CONTRIBUTING.md] for details

[CONTRIBUTING.md]: https://github.com/nushell/nushell/blob/main/crates/nu-std/CONTRIBUTING.md
