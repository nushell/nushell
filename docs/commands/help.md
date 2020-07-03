# help

Use `help` for more information on a command.
Use `help commands` to list all available commands.
Use `help <command name>` to display help about a particular command.

## Examples

```shell
> help
Welcome to Nushell.

Here are some tips to help you get started.
  * help commands - list all available commands
  * help <command name> - display help about a particular command

Nushell works on the idea of a "pipeline". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

[Examples]

List the files in the current directory, sorted by size:
    ls | sort-by size

Get information about the current system:
    sys | get host

Get the processes on your system actively using CPU:
    ps | where cpu > 0

You can also learn more at https://www.nushell.sh/book/
```

```shell
> help commands
────┬──────────────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 #  │ name         │ description
────┼──────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 0  │ alias        │ Define a shortcut for another command.
 1  │ append       │ Append the given row to the table
 2  │ autoview     │ View the contents of the pipeline as a table or list.
 3  │ build-string │ Builds a string from the arguments
 4  │ cal          │ Display a calendar.
 5  │ calc         │ Parse a math expression into a number
...
 83 │ where        │ Filter table to match the condition.
 84 │ which        │ Finds a program file.
 85 │ with-env     │ Runs a block with an environment set. Eg) with-env [NAME 'foo'] { echo $nu.env.NAME }
 86 │ wrap         │ Wraps the given data in a table.
────┴──────────────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
```

```shell
> help cd
Change to a new path.

Usage:
  > cd (directory) {flags}

Parameters:
  (directory) the directory to change to

Flags:
  -h, --help: Display this help message

Examples:
  Change to a new directory called 'dirname'
  > cd dirname

  Change to your home directory
  > cd

  Change to your home directory (alternate version)
  > cd ~

  Change to the previous directory
  > cd -
```
