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
> shells | str upcase path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /HOME/TUX/STUFF/EXPR/STUFF
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> shells | str downcase path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /home/tux/stuff/expr/stuff
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> shells | str substring "21, 99" path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ stuff
 1 │   │ filesystem │
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> shells | str substring "6," path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ TUX/stuff/expr/stuff
 1 │   │ filesystem │
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

> echo "1, 2, 3" | split row "," | str to-int | sum
━━━━━━━━━
 <value>
─────────
       6
━━━━━━━━━

> echo "nu" | str capitalize
━━━━━━━━━
 <value>
─────────
      Nu
━━━━━━━━━

> echo "Nu    " | str trim
━━━━━━━━━
 <value>
─────────
      Nu
━━━━━━━━━
> shells | str find-replace "TUX" "skipper" path
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /home/skipper/stuff/expr/stuff
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
