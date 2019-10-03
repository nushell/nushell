# add

This command adds a column to any table output. The first parameter takes the heading, the second parameter takes the value for all the rows.

## Examples

```shell
> ls | add is_on_a_computer yes_obviously
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━
 # │ name                       │ type │ readonly │ size   │ accessed  │ modified  │ is_on_a_computer 
───┼────────────────────────────┼──────┼──────────┼────────┼───────────┼───────────┼──────────────────
 0 │ zeusiscrazy.txt            │ File │          │ 556 B  │ a day ago │ a day ago │ yes_obviously 
 1 │ coww.txt                   │ File │          │  24 B  │ a day ago │ a day ago │ yes_obviously 
 2 │ randomweirdstuff.txt       │ File │          │ 197 B  │ a day ago │ a day ago │ yes_obviously 
 3 │ abaracadabra.txt           │ File │          │ 401 B  │ a day ago │ a day ago │ yes_obviously 
 4 │ youshouldeatmorecereal.txt │ File │          │ 768 B  │ a day ago │ a day ago │ yes_obviously 
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━
```

```shell
> shells | add os linux_on_this_machine
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path                           │ os 
───┼───┼────────────┼────────────────────────────────┼───────────────────────
 0 │ X │ filesystem │ /home/shaurya/stuff/expr/stuff │ linux_on_this_machine 
 1 │   │ filesystem │ /                              │ linux_on_this_machine 
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━
```
