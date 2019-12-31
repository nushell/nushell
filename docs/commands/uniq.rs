# uniq

Returns unique rows or values from a dataset.

## Examples

Given a file `test.csv`

```
first_name,last_name,rusty_at,type
Andrés,Robalino,10/11/2013,A
Andrés,Robalino,10/11/2013,A
Jonathan,Turner,10/12/2013,B
Yehuda,Katz,10/11/2013,A
```

```
> `open test.csv | uniq`
━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━━┯━━━━━━
 # │ first_name │ last_name │ rusty_at   │ type
───┼────────────┼───────────┼────────────┼──────
0 │ Andrés     │ Robalino  │ 10/11/2013 │ A
1 │ Jonathan   │ Turner    │ 10/12/2013 │ B
2 │ Yehuda     │ Katz      │ 10/11/2013 │ A
━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━━┷━━━━━━
```

```
> `open test.csv | get type | uniq`
━━━┯━━━━━━━━━
# │ <value>
───┼─────────
0 │ A
1 │ B
━━━┷━━━━━━━━━
```
