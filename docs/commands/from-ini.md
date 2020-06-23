# from ini

Converts ini data into table. Use this when nushell cannot determine the input file extension.

## Example

Let's say we have the following `.txt` file:

```shell
> open sample.txt
[SectionOne]

key = value
integer = 1234
string1 = 'Case 1'
```

This file is actually a ini file, but the file extension isn't `.ini`. That's okay, we can use the `from ini` command:

```shell
> open sample.txt | from ini | get SectionOne
━━━━━━━┯━━━━━━━━━┯━━━━━━━━━━
 key   │ integer │ string1
───────┼─────────┼──────────
 value │ 1234    │ 'Case 1'
━━━━━━━┷━━━━━━━━━┷━━━━━━━━━━
```
