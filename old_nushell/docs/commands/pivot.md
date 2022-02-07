# pivot

Pivots the table contents so rows become columns and columns become rows.

## Examples

```shell
> ls docs
───┬────────────────────┬──────┬────────┬─────────────
 # │ name               │ type │ size   │ modified
───┼────────────────────┼──────┼────────┼─────────────
 0 │ docs/commands      │ Dir  │ 2.7 KB │ 53 mins ago
 1 │ docs/docker.md     │ File │ 7.0 KB │ 40 mins ago
 2 │ docs/philosophy.md │ File │  912 B │ 54 mins ago
───┴────────────────────┴──────┴────────┴─────────────
```

```shell
> ls docs | pivot
───┬──────────┬───────────────┬────────────────┬────────────────────
 # │ Column0  │ Column1       │ Column2        │ Column3
───┼──────────┼───────────────┼────────────────┼────────────────────
 0 │ name     │ docs/commands │ docs/docker.md │ docs/philosophy.md
 1 │ type     │ Dir           │ File           │ File
 2 │ size     │        2.7 KB │         7.0 KB │              912 B
 3 │ modified │ 53 mins ago   │ 40 mins ago    │ 55 mins ago
───┴──────────┴───────────────┴────────────────┴────────────────────
```

Use `--header-row` to treat the first row as column names:

```shell
> ls docs | pivot --header-row
───┬───────────────┬────────────────┬────────────────────
 # │ docs/commands │ docs/docker.md │ docs/philosophy.md
───┼───────────────┼────────────────┼────────────────────
 0 │ Dir           │ File           │ File
 1 │        2.7 KB │         7.0 KB │              912 B
 2 │ 53 mins ago   │ 40 mins ago    │ 55 mins ago
───┴───────────────┴────────────────┴────────────────────
```

Use `--ignore-titles` to prevent pivoting the column names into values:

```shell
> ls docs | pivot --ignore-titles
───┬───────────────┬────────────────┬────────────────────
 # │ Column0       │ Column1        │ Column2
───┼───────────────┼────────────────┼────────────────────
 0 │ docs/commands │ docs/docker.md │ docs/philosophy.md
 1 │ Dir           │ File           │ File
 2 │        2.7 KB │         7.0 KB │              912 B
 3 │ 54 mins ago   │ 41 mins ago    │ 56 mins ago
───┴───────────────┴────────────────┴────────────────────
```

Additional arguments are used as column names:

```shell
> ls docs | pivot foo bar baz
───┬──────────┬───────────────┬────────────────┬────────────────────
 # │ foo      │ bar           │ baz            │ Column3
───┼──────────┼───────────────┼────────────────┼────────────────────
 0 │ name     │ docs/commands │ docs/docker.md │ docs/philosophy.md
 1 │ type     │ Dir           │ File           │ File
 2 │ size     │        2.7 KB │         7.0 KB │              912 B
 3 │ modified │ 55 mins ago   │ 41 mins ago    │ 56 mins ago
───┴──────────┴───────────────┴────────────────┴────────────────────
```
