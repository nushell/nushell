# du

`du` stands for disk usage. It will give you the physical and apparent size of files and folders

## Examples

```shell
> du src/commands
───┬──────────────┬──────────┬──────────┬────────────────
 # │ path         │ apparent │ physical │ directories
───┼──────────────┼──────────┼──────────┼────────────────
 0 │ src/commands │ 411.5 KB │ 647.2 KB │ [table 1 rows]
───┴──────────────┴──────────┴──────────┴────────────────
> du -a src/commands
───┬──────────────┬──────────┬──────────┬─────────────────┬────────────────
 # │ path         │ apparent │ physical │ files           │ directories
───┼──────────────┼──────────┼──────────┼─────────────────┼────────────────
 0 │ src/commands │ 411.5 KB │ 647.2 KB │ [table 95 rows] │ [table 1 rows]
───┴──────────────┴──────────┴──────────┴─────────────────┴────────────────
> du *.rs
───┬──────────┬──────────┬──────────
 # │ path     │ apparent │ physical
───┼──────────┼──────────┼──────────
 0 │ build.rs │     78 B │   4.1 KB
───┴──────────┴──────────┴──────────
```