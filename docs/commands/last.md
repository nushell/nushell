# last

Use `last` to retrieve the last "n" rows of a table. `last` has a required amount parameter that indicates how many rows you would like returned. If more than one row is returned, an index column will be included showing the row number. `last` does not alter the order of the rows of the table.

## Examples

```shell
> ps | last 1
─────────┬─────────────
 pid     │ 167
 name    │ loginwindow
 status  │ Running
 cpu     │ 0.0000
 mem     │ 461.2 MB
 virtual │ 7.2 GB
─────────┴─────────────
```

```shell
> ps | last 5
───┬─────┬─────────────────┬─────────┬────────┬──────────┬─────────
 # │ pid │ name            │ status  │ cpu    │ mem      │ virtual
───┼─────┼─────────────────┼─────────┼────────┼──────────┼─────────
 0 │ 334 │ knowledge-agent │ Running │ 0.0000 │  53.7 MB │  6.7 GB
 1 │ 332 │ UserEventAgent  │ Running │ 0.0000 │  22.1 MB │  6.6 GB
 2 │ 326 │ cfprefsd        │ Running │ 0.0000 │   8.1 MB │  5.6 GB
 3 │ 325 │ coreauthd       │ Running │ 0.0000 │   9.7 MB │  5.0 GB
 4 │ 167 │ loginwindow     │ Running │ 0.0000 │ 461.2 MB │  7.2 GB
───┴─────┴─────────────────┴─────────┴────────┴──────────┴─────────
```
