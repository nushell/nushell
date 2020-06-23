# nth

This command returns the nth row of a table, starting from 0.
If the number given is less than 0 or more than the number of rows, nothing is returned.

## Usage

```shell
> [input-command] | nth <row number>  ...args
```

## Parameters

* `<row number>` the number of the row to return
* `args`: Optionally return more rows

## Examples

```shell
> ls
────┬────────────────────┬──────┬──────────┬──────────────
 #  │ name               │ type │ size     │ modified
────┼────────────────────┼──────┼──────────┼──────────────
 0  │ CODE_OF_CONDUCT.md │ File │   3.4 KB │ 53 mins ago
 1  │ CONTRIBUTING.md    │ File │   1.3 KB │ 6 mins ago
 2  │ Cargo.lock         │ File │ 113.3 KB │ 53 mins ago
 3  │ Cargo.toml         │ File │   4.6 KB │ 53 mins ago
 4  │ LICENSE            │ File │   1.1 KB │ 3 months ago
 5  │ Makefile.toml      │ File │    449 B │ 5 months ago
 6  │ README.md          │ File │  15.8 KB │ 2 mins ago
 7  │ TODO.md            │ File │      0 B │ 53 mins ago
 8  │ assets             │ Dir  │    128 B │ 5 months ago
 9  │ build.rs           │ File │     78 B │ 4 months ago
 10 │ crates             │ Dir  │    704 B │ 53 mins ago
 11 │ debian             │ Dir  │    352 B │ 5 months ago
 12 │ docker             │ Dir  │    288 B │ 3 months ago
 13 │ docs               │ Dir  │    192 B │ 53 mins ago
 14 │ features.toml      │ File │    632 B │ 4 months ago
 15 │ images             │ Dir  │    160 B │ 5 months ago
 16 │ rustfmt.toml       │ File │     16 B │ 5 months ago
 17 │ src                │ Dir  │    128 B │ 1 day ago
 18 │ target             │ Dir  │    160 B │ 5 days ago
 19 │ tests              │ Dir  │    192 B │ 3 months ago
────┴────────────────────┴──────┴──────────┴──────────────
```

```shell
> ls | nth 0
──────────┬────────────────────
 name     │ CODE_OF_CONDUCT.md
 type     │ File
 size     │ 3.4 KB
 modified │ 54 mins ago
──────────┴────────────────────
```

```shell
> ls | nth 0 2
───┬────────────────────┬──────┬──────────┬─────────────
 # │ name               │ type │ size     │ modified
───┼────────────────────┼──────┼──────────┼─────────────
 0 │ CODE_OF_CONDUCT.md │ File │   3.4 KB │ 54 mins ago
 1 │ Cargo.lock         │ File │ 113.3 KB │ 54 mins ago
───┴────────────────────┴──────┴──────────┴─────────────
```

```shell
> ls | nth 5
──────────┬───────────────
 name     │ Makefile.toml
 type     │ File
 size     │ 449 B
 modified │ 5 months ago
──────────┴───────────────
```
