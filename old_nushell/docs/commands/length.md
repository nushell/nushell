# length

Obtain the row or column count of a table.

## Flags

* `-c`, `--column`: Calculate number of columns in table

## Examples

```shell
> ls
────┬────────────────────┬──────┬──────────┬──────────────
 #  │ name               │ type │ size     │ modified
────┼────────────────────┼──────┼──────────┼──────────────
 0  │ CODE_OF_CONDUCT.md │ File │   3.4 KB │ 42 mins ago
 1  │ CONTRIBUTING.md    │ File │   1.3 KB │ 42 mins ago
 2  │ Cargo.lock         │ File │ 113.3 KB │ 42 mins ago
 3  │ Cargo.toml         │ File │   4.6 KB │ 42 mins ago
 4  │ LICENSE            │ File │   1.1 KB │ 3 months ago
 5  │ Makefile.toml      │ File │    449 B │ 5 months ago
 6  │ README.md          │ File │  15.9 KB │ 31 mins ago
 7  │ TODO.md            │ File │      0 B │ 42 mins ago
 8  │ assets             │ Dir  │    128 B │ 5 months ago
 9  │ build.rs           │ File │     78 B │ 4 months ago
 10 │ crates             │ Dir  │    704 B │ 42 mins ago
 11 │ debian             │ Dir  │    352 B │ 5 months ago
 12 │ docker             │ Dir  │    288 B │ 3 months ago
 13 │ docs               │ Dir  │    192 B │ 42 mins ago
 14 │ features.toml      │ File │    632 B │ 4 months ago
 15 │ images             │ Dir  │    160 B │ 5 months ago
 16 │ rustfmt.toml       │ File │     16 B │ 5 months ago
 17 │ src                │ Dir  │    128 B │ 1 day ago
 18 │ target             │ Dir  │    160 B │ 5 days ago
 19 │ tests              │ Dir  │    192 B │ 3 months ago
────┴────────────────────┴──────┴──────────┴──────────────
```

By default, `length` will return the number of rows in a table

```shell
> ls | length
20
```

The `-c` flag will produce a count of the columns in the table

```shell
> ls | length -c
4
```

```shell
> ls | where type == File | length
11
```

```shell
> ls | where type == Dir | length
9
```

```shell
> ls | where size > 2KB | length
4
```
