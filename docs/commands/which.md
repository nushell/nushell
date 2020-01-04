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
/home/bob> which python
━━━━━━━━┯━━━━━━━━━━━━━━━━━┯━━━━━━━━━
 arg    │ path            │ builtin
────────┼─────────────────┼─────────
 python │ /usr/bin/python │ No
━━━━━━━━┷━━━━━━━━━━━━━━━━━┷━━━━━━━━━
/home/bob> which cargo
━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━
 arg   │ path                       │ builtin
───────┼────────────────────────────┼─────────
 cargo │ /home/bob/.cargo/bin/cargo │ No
━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━
```

`which` will identify nushell commands:

```shell
/home/bob> which ls
━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━
 arg │ path                     │ builtin
─────┼──────────────────────────┼─────────
 ls  │ nushell built-in command │ Yes
━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━
/home/bob> which which
━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━
 arg   │ path                     │ builtin
───────┼──────────────────────────┼─────────
 which │ nushell built-in command │ Yes
━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━
```

Passing the `all` flag identifies all instances of a command or binary

```shell
/home/bob> which ls --all
━━━┯━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━
 # │ arg │ path                     │ builtin
───┼─────┼──────────────────────────┼─────────
 0 │ ls  │ nushell built-in command │ Yes
 1 │ ls  │ /usr/bin/ls              │ No
━━━┷━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━
```

`which` will also identify local binaries

```shell
/home/bob> touch foo
/home/bob> chmod +x foo
/home/bob> which ./foo
━━━━━━━┯━━━━━━━━━━━━━━━┯━━━━━━━━━
 arg   │ path          │ builtin
───────┼───────────────┼─────────
 ./foo │ /home/bob/foo │ No
━━━━━━━┷━━━━━━━━━━━━━━━┷━━━━━━━━━
```
