# du

`du` stands for disk usage. It will give you the physical and apparent size of files and folders

## Examples

```shell
> du src/commands
─────────────┬────────────────────────────
 path        │ crates/nu-cli/src/commands
 apparent    │ 655.9 KB
 physical    │ 950.3 KB
 directories │ [table 5 rows]
 files       │
─────────────┴────────────────────────────
```

```shell
> du -a src/commands
─────────────┬────────────────────────────
 path        │ crates/nu-cli/src/commands
 apparent    │ 655.9 KB
 physical    │ 950.3 KB
 directories │ [table 5 rows]
 files       │ [table 118 rows]
─────────────┴────────────────────────────
```

```shell
> du *.rs
───┬──────────┬──────────┬──────────
 # │ path     │ apparent │ physical
───┼──────────┼──────────┼──────────
 0 │ build.rs │     78 B │   4.1 KB
───┴──────────┴──────────┴──────────
```
