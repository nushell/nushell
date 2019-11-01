# str

Consumes either a single value or a table and converts the provided data to a string and optionally applies a change.

## Examples

```shell
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /home/TUX/stuff/expr/stuff
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> shells | str path --upcase
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /HOME/TUX/STUFF/EXPR/STUFF
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> shells | str path --downcase
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /home/tux/stuff/expr/stuff
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> shells | str # --substring "21, 99"
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ stuff
 1 │   │ filesystem │
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> shells | str # --substring "6,"
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ TUX/stuff/expr/stuff
 1 │   │ filesystem │
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

> echo "1, 2, 3" | split-row "," | str --to-int | sum
━━━━━━━━━
 <value>
─────────
       6
━━━━━━━━━
```
