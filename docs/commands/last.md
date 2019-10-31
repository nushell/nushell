# last

Use `last` to retrieve the last "n" rows of a table. `last` has a required amount parameter that indicates how many rows you would like returned. If more than one row is returned, an index column will be included showing the row number. `last` does not alter the order of the rows of the table.

## Examples

```shell
> ps | last 1
━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━
 pid │ name        │ status  │ cpu
─────┼─────────────┼─────────┼───────────────────
 121 │ loginwindow │ Running │ 0.000000000000000
━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━
```

```shell
> ps | last 5
━━━┯━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━
 # │ pid │ name           │ status  │ cpu
───┼─────┼────────────────┼─────────┼───────────────────
 0 │ 360 │ CommCenter     │ Running │ 0.000000000000000
 1 │ 358 │ distnoted      │ Running │ 0.000000000000000
 2 │ 356 │ UserEventAgent │ Running │ 0.000000000000000
 3 │ 354 │ cfprefsd       │ Running │ 0.000000000000000
 4 │ 121 │ loginwindow    │ Running │ 0.000000000000000
━━━┷━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━
```


