# from json

Parse text as `.json` and create table. Use this when nushell cannot determine the input file extension.

Syntax: `from json {flags}`

## Flags

    --objects
      treat each line as a separate value

## Examples

```shell
> open command_from-json
[
    {
        title: "from json",
        type: "command",
        flags: true
    }
]
```

```shell
> open command_from-json | from json
━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━━
 title     │ type    │ flags
───────────┼─────────┼───────
 from json │ command │ true
━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━━
```
