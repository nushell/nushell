# first

Use `first` to retrieve the first "n" rows of a table. `first` has a required amount parameter that indicates how many rows you would like returned. If more than one row is returned, an index column will be included showing the row number.

## Examples

```shell
> ps | first 1
━━━━━━━┯━━━━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━
 pid   │ name         │ status  │ cpu
───────┼──────────────┼─────────┼───────────────────
 60358 │ nu_plugin_ps │ Running │ 5.399802999999999
━━━━━━━┷━━━━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━
```

```shell
> ps | first 5
━━━┯━━━━━━━┯━━━━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━
 # │ pid   │ name         │ status  │ cpu
───┼───────┼──────────────┼─────────┼───────────────────
 0 │ 60754 │ nu_plugin_ps │ Running │ 4.024156000000000
 1 │ 60107 │ quicklookd   │ Running │ 0.000000000000000
 2 │ 59356 │ nu           │ Running │ 0.000000000000000
 3 │ 59216 │ zsh          │ Running │ 0.000000000000000
 4 │ 59162 │ vim          │ Running │ 0.000000000000000
━━━┷━━━━━━━┷━━━━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━
```

