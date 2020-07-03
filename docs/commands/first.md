# first

Use `first` to retrieve the first "n" rows of a table. `first` has a required amount parameter that indicates how many rows you would like returned. If more than one row is returned, an index column will be included showing the row number.

## Examples

```shell
> ps | first 1
─────────┬──────────────────
 pid     │ 14733
 name    │ nu_plugin_core_p
 status  │ Running
 cpu     │ 4.1229
 mem     │ 2.1 MB
 virtual │ 4.8 GB
─────────┴──────────────────

```

```shell
> ps | first 5
───┬───────┬──────────────────┬─────────┬──────────┬─────────┬─────────
 # │ pid   │ name             │ status  │ cpu      │ mem     │ virtual
───┼───────┼──────────────────┼─────────┼──────────┼─────────┼─────────
 0 │ 14747 │ nu_plugin_core_p │ Running │   3.5653 │  2.1 MB │  4.8 GB
 1 │ 14735 │ Python           │ Running │ 100.0008 │ 27.4 MB │  5.4 GB
 2 │ 14734 │ mdworker_shared  │ Running │   0.0000 │ 18.4 MB │  4.7 GB
 3 │ 14729 │ mdworker_shared  │ Running │   0.0000 │  8.2 MB │  5.0 GB
 4 │ 14728 │ mdworker_shared  │ Running │   0.0000 │  8.0 MB │  4.9 GB
───┴───────┴──────────────────┴─────────┴──────────┴─────────┴─────────
```
