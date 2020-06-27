# autoview

Print the content of the pipeline as a table or list.
It is the implied or default output when none is provided.

`-h`, `--help`
  Display help message.

## Examples

```shell
> ls | autoview
────┬────────────────────┬──────┬─────────┬──────────────
 #  │ name               │ type │ size    │ modified
────┼────────────────────┼──────┼─────────┼──────────────
  0 │ README.md          │ File │   932 B │ 19 hours ago
  1 │ alias.md           │ File │  2.0 KB │ 19 hours ago
  2 │ append.md          │ File │  1.4 KB │ 19 hours ago
   ...
 82 │ wrap.md            │ File │  1.8 KB │ 19 hours ago
────┴────────────────────┴──────┴─────────┴──────────────
```

Note that `ls` (and most commands) produces the same output as `ls | autoview` since `autoview` is implied.
