# pwd

Print the current working directory.

`-h`, `--help`
  Display help message.

## Examples

```shell
> pwd
/home/me/nushell/docs/commands
```

```shell
> pwd | split column "/" | reject Column1 | pivot | reject Column0
───┬──────────
 # │ Column1
───┼──────────
 0 │ home
 1 │ me
 2 │ projects
 3 │ nushell
 4 │ docs
 5 │ commands
───┴──────────
```
