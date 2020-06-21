# uniq

Returns unique rows or values from a dataset.

## Examples

Given a file `test.csv`

```csv
first_name,last_name,rusty_at,type
Andrés,Robalino,10/11/2013,A
Andrés,Robalino,10/11/2013,A
Jonathan,Turner,10/12/2013,B
Yehuda,Katz,10/11/2013,A
```

```shell
> `open test.csv | uniq`
━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━━┯━━━━━━
 # │ first_name │ last_name │ rusty_at   │ type
───┼────────────┼───────────┼────────────┼──────
 0 │ Andrés     │ Robalino  │ 10/11/2013 │ A
 1 │ Jonathan   │ Turner    │ 10/12/2013 │ B
 2 │ Yehuda     │ Katz      │ 10/11/2013 │ A
━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━━┷━━━━━━
```

```shell
> `open test.csv | get type | uniq`
━━━┯━━━━━━━━━
 # │
───┼─────────
 0 │ A
 1 │ B
━━━┷━━━━━━━━━
```

### Counting

`--count` or `-c` is the flag to output a `count` column.

```shell
> `open test.csv | get type | uniq -c`
───┬───────┬───────
 # │ value │ count
───┼───────┼───────
 0 │ A     │     3
 1 │ B     │     2
───┴───────┴───────
```
