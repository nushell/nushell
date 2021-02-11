# which

Finds a program file.

Usage:
  > which <application> {flags}

## Parameters

- application: the name of the command to find the path to

## Flags

- --all: list all executables

## Examples

`which` finds the location of an executable:

```shell
> which python
─────────┬─────────────────
 arg     │ python
 path    │ /usr/bin/python
 builtin │ false
─────────┴─────────────────
```

```shell
> which cargo
─────────┬────────────────────────────
 arg     │ cargo
 path    │ /home/bob/.cargo/bin/cargo
 builtin │ false
─────────┴────────────────────────────
```

`which` will identify nushell commands:

```shell
> which ls
─────────┬──────────────────────────
 arg     │ ls
 path    │ nushell built-in command
 builtin │ true
─────────┴──────────────────────────
```

```shell
> which which
─────────┬──────────────────────────
 arg     │ which
 path    │ nushell built-in command
 builtin │ true
─────────┴──────────────────────────
```

Passing the `all` flag identifies all instances of a command or binary

```shell
> which ls --all
───┬─────┬──────────────────────────┬─────────
 # │ arg │ path                     │ builtin
───┼─────┼──────────────────────────┼─────────
 0 │ ls  │ nushell built-in command │ true
 1 │ ls  │ /bin/ls                  │ false
───┴─────┴──────────────────────────┴─────────
```

`which` will also identify local binaries

```shell
> touch foo
> chmod +x foo
> which ./foo
─────────┬────────────────────────────────
 arg     │ ./foo
 path    │ /Users/josephlyons/Desktop/foo
 builtin │ false
─────────┴────────────────────────────────
```

`which` also identifies aliases

```shell
> alias e = echo
> which e
───┬─────┬───────────────┬─────────
 # │ arg │     path      │ builtin
───┼─────┼───────────────┼─────────
 0 │ e   │ Nushell alias │ false
───┴─────┴───────────────┴─────────
```

and custom commands

```shell
> def my_cool_echo [arg] { echo $arg }
> which my_cool_echo
───┬──────────────┬────────────────────────┬─────────
 # │     arg      │          path          │ builtin
───┼──────────────┼────────────────────────┼─────────
 0 │ my_cool_echo │ Nushell custom command │ false
───┴──────────────┴────────────────────────┴─────────
```
