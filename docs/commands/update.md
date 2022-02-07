# update

Updates an existing column on a table. First parameter is the column to update and the second parameter is the value to put.

## Examples

```shell
> ls
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━
 # │ name                       │ type │ readonly │ size   │ accessed  │ modified
───┼────────────────────────────┼──────┼──────────┼────────┼───────────┼───────────
 0 │ zeusiscrazy.txt            │ File │          │ 556 B  │ a day ago │ a day ago
 1 │ coww.txt                   │ File │          │  24 B  │ a day ago │ a day ago
 2 │ randomweirdstuff.txt       │ File │          │ 197 B  │ a day ago │ a day ago
 3 │ abaracadabra.txt           │ File │          │ 401 B  │ a day ago │ a day ago
 4 │ youshouldeatmorecereal.txt │ File │          │ 768 B  │ a day ago │ a day ago
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━
```

```shell
> ls | update modified neverrrr
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━
 # │ name                       │ type │ readonly │ size   │ accessed  │ modified
───┼────────────────────────────┼──────┼──────────┼────────┼───────────┼──────────
 0 │ zeusiscrazy.txt            │ File │          │ 556 B  │ a day ago │ neverrrr
 1 │ coww.txt                   │ File │          │  24 B  │ a day ago │ neverrrr
 2 │ randomweirdstuff.txt       │ File │          │ 197 B  │ a day ago │ neverrrr
 3 │ abaracadabra.txt           │ File │          │ 401 B  │ a day ago │ neverrrr
 4 │ youshouldeatmorecereal.txt │ File │          │ 768 B  │ a day ago │ neverrrr
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━
```

```shell
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────────────
 0 │ X │ filesystem │ /home/username/stuff/expr/stuff
 1 │   │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
> shells | update " " X | update path /
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼──────
 0 │ X │ filesystem │ /
 1 │ X │ filesystem │ /
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━
```

Collect all the values of a nested column and join them together
```shell
> version | update features {get features | str collect ', '}
───┬─────────┬──────────────────────────────────────────┬───────────────────────────
 # │ version │               commit_hash                │         features
───┼─────────┼──────────────────────────────────────────┼───────────────────────────
 0 │ 0.20.0  │ fdab3368094e938c390f1e5a7892a42da45add3e │ default, clipboard, trash
───┴─────────┴──────────────────────────────────────────┴───────────────────────────
