# str

Applies the subcommand to a value or a table.

## Examples

```shell
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /home/TUX/stuff/expr/stuff
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
> shells | str upcase path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /HOME/TUX/STUFF/EXPR/STUFF
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
> shells | str downcase path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /home/tux/stuff/expr/stuff
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
> shells | str substring "21, 99" path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ stuff
 1 │   │ filesystem │
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
> shells | str substring "6," path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ TUX/stuff/expr/stuff
 1 │   │ filesystem │
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
> echo "1, 2, 3" | split row "," | str to-int | math sum
6
```

```shell
> echo "nu" | str capitalize
Nu
```

```shell
> echo "Nu    " | str trim
Nu
```

```shell
> echo "Nushell" | str reverse
llehsuN
```

```shell
> shells | str find-replace "TUX" "skipper" path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /home/skipper/stuff/expr/stuff
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```
