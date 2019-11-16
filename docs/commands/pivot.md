# pivot

Pivots the table contents so rows become columns and columns become rows.

## Examples

```sh
> ls docs
━━━┯━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━━━━━
 # │ name               │ type      │ readonly │ size   │ accessed    │ modified
───┼────────────────────┼───────────┼──────────┼────────┼─────────────┼─────────────
 0 │ docs/commands      │ Directory │          │ 4.1 KB │ an hour ago │ an hour ago
 1 │ docs/docker.md     │ File      │          │ 7.0 KB │ an hour ago │ a day ago
 2 │ docs/philosophy.md │ File      │          │ 896 B  │ an hour ago │ a day ago
━━━┷━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━━━━━

> ls docs | pivot
━━━┯━━━━━━━━━━┯━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━
 # │ Column0  │ Column1       │ Column2        │ Column3
───┼──────────┼───────────────┼────────────────┼────────────────────
 0 │ name     │ docs/commands │ docs/docker.md │ docs/philosophy.md
 1 │ type     │ Directory     │ File           │ File
 2 │ readonly │               │                │
 3 │ size     │        4.1 KB │         7.0 KB │             896 B
 4 │ accessed │ an hour ago   │ an hour ago    │ an hour ago
 5 │ modified │ an hour ago   │ a day ago      │ a day ago
━━━┷━━━━━━━━━━┷━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━
```

Use `--header-row` to treat the first row as column names:

```shell
> ls docs | pivot --header-row
━━━┯━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━
 # │ docs/commands │ docs/docker.md │ docs/philosophy.md
───┼───────────────┼────────────────┼────────────────────
 0 │ Directory     │ File           │ File
 1 │               │                │
 2 │        4.1 KB │         7.0 KB │             896 B
 3 │ an hour ago   │ an hour ago    │ an hour ago
 4 │ an hour ago   │ a day ago      │ a day ago
━━━┷━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━
```

Use `--ignore-titles` to prevent pivoting the column names into values:

```shell
> ls docs | pivot --ignore-titles
━━━┯━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━
 # │ Column0       │ Column1        │ Column2
───┼───────────────┼────────────────┼────────────────────
 0 │ docs/commands │ docs/docker.md │ docs/philosophy.md
 1 │ Directory     │ File           │ File
 2 │               │                │
 3 │        4.1 KB │         7.0 KB │             896 B
 4 │ an hour ago   │ an hour ago    │ an hour ago
 5 │ an hour ago   │ a day ago      │ a day ago
━━━┷━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━
```

Additional arguments are used as column names:

```shell
> ls docs | pivot foo bar baz
━━━┯━━━━━━━━━━┯━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━
 # │ foo      │ bar           │ baz            │ Column3
───┼──────────┼───────────────┼────────────────┼────────────────────
 0 │ name     │ docs/commands │ docs/docker.md │ docs/philosophy.md
 1 │ type     │ Directory     │ File           │ File
 2 │ readonly │               │                │
 3 │ size     │        4.1 KB │         7.0 KB │             896 B
 4 │ accessed │ 2 hours ago   │ 2 hours ago    │ 2 hours ago
 5 │ modified │ 2 hours ago   │ a day ago      │ a day ago
━━━┷━━━━━━━━━━┷━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━
```
