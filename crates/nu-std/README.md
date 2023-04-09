<h1 align="center">
  Welcome to the standard library of `nushell`!
  <img src="https://media.giphy.com/media/hvRJCLFzcasrR4ia7z/giphy.gif" width="28"></img>
</h1>

The standard library is a pure-`nushell` collection of custom commands which 
provide interactive utilities and building blocks for users writing casual scripts or complex applications.

To see what's here:
```
〉use std
〉help commands | select name usage | where name =~ "std "
╭────┬─────────────────────────────┬─────────────────────────────────────────────────────────────────────╮
│  # │            name             │                                usage                                │
│  0 │ std assert                  │ Universal assert command                                            │
│  1 │ std assert equal            │ Assert $left == $right                                              │
│  2 │ std assert error            │ Assert that executing the code generates an error                   │
│  3 │ std assert greater          │ Assert $left > $right                                               │
│  4 │ std assert greater or equal │ Assert $left >= $right                                              │
│  5 │ std assert length           │ Assert length of $left is $right                                    │
│  6 │ std assert less             │ Assert $left < $right                                               │
│  7 │ std assert less or equal    │ Assert $left <= $right                                              │
│  8 │ std assert not equal        │ Assert $left != $right                                              │
│  9 │ std assert skip             │ Skip the current test case                                          │
│ 10 │ std assert str contains     │ Assert that ($left | str contains $right)                           │
│ 11 │ std clip                    │ put the end of a pipe into the system clipboard.                    │
│ 12 │ std dirs add                │ Add one or more directories to the list.                            │
│    │                             │ PWD becomes first of the newly added directories.                   │
│ 13 │ std dirs drop               │ Drop the current directory from the list, if it's not the only one. │
│    │                             │ PWD becomes the next working directory                              │
│ 14 │ std dirs next               │ Advance to the next directory in the list or wrap to beginning.     │
│ 15 │ std dirs prev               │ Back up to the previous directory or wrap to the end.               │
│ 16 │ std dirs show               │ Display current working directories.                                │
│ 17 │ std help                    │ Display help information about different parts of Nushell.          │
│ 18 │ std help aliases            │ Show help on nushell aliases.                                       │
│ 19 │ std help commands           │ Show help on nushell commands.                                      │
│ 20 │ std help externs            │ Show help on nushell externs.                                       │
│ 21 │ std help modules            │ Show help on nushell modules.                                       │
│ 22 │ std help operators          │ Show help on nushell operators.                                     │
│ 23 │ std log critical            │ Log critical message                                                │
│ 24 │ std log debug               │ Log debug message                                                   │
│ 25 │ std log error               │ Log error message                                                   │
│ 26 │ std log info                │ Log info message                                                    │
│ 27 │ std log warning             │ Log warning message                                                 │
│ 28 │ std path add                │ Add the given paths to the PATH.                                    │
│ 29 │ std xaccess                 │ Get all xml entries matching simple xpath-inspired query            │
│ 30 │ std xinsert                 │ Insert new entry to elements matching simple xpath-inspired query   │
│ 31 │ std xtype                   │ Get type of an xml entry                                            │
│ 32 │ std xupdate                 │ Update xml data entries matching simple xpath-inspired query        │
├────┼─────────────────────────────┼─────────────────────────────────────────────────────────────────────┤
│  # │            name             │                                usage                                │
╰────┴─────────────────────────────┴─────────────────────────────────────────────────────────────────────╯

```

## :toolbox: Using the standard library in the REPL or in scripts
All commands in the standard library must be "imported" into the running environment 
(the interactive read-execute-print-loop (REPL) or a `.nu` script) using the
[`use`](https://nushell.sh/commands/docs/use.html) command.

You can choose to import the whole module, but then must refer to invidual commands with a `std` prefix, e.g:
```
use std
 . . .
std log debug "Running now"
std assert (1 == 2)
```

Or you can enumerate the specific commands you want to import and invoke them without the `std` prefix.
```
use std log assert
. . .
log debug "Running again"
assert (2 == 1)
```
This is probably the form of import you'll want to add to your `env.nu` for interactive use.

## :pencil2: contribute to the standard library
You're invited to contribute to the standard library! 
See [CONTRIBUTING.md](./CONTRIBUTING.md) for details
