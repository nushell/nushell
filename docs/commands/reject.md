# reject

This command removes or rejects the columns passed to it.

## Examples 

```shell
> ls
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━━━━━
 # │ name                       │ type │ readonly │ size   │ created     │ accessed    │ modified 
───┼────────────────────────────┼──────┼──────────┼────────┼─────────────┼─────────────┼─────────────
 0 │ zeusiscrazy.txt            │ File │          │ 556 B  │ a month ago │ a month ago │ a month ago 
 1 │ coww.txt                   │ File │          │  24 B  │ a month ago │ a month ago │ a month ago 
 2 │ randomweirdstuff.txt       │ File │          │ 197 B  │ a month ago │ a month ago │ a month ago 
 3 │ abaracadabra.txt           │ File │          │ 401 B  │ a month ago │ a month ago │ a month ago 
 4 │ youshouldeatmorecereal.txt │ File │          │ 768 B  │ a month ago │ a month ago │ a month ago 
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━━━━━
> ls | reject readonly
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━━━━━
 # │ name                       │ type │ size   │ created     │ accessed    │ modified 
───┼────────────────────────────┼──────┼────────┼─────────────┼─────────────┼─────────────
 0 │ zeusiscrazy.txt            │ File │ 556 B  │ a month ago │ a month ago │ a month ago 
 1 │ coww.txt                   │ File │  24 B  │ a month ago │ a month ago │ a month ago 
 2 │ randomweirdstuff.txt       │ File │ 197 B  │ a month ago │ a month ago │ a month ago 
 3 │ abaracadabra.txt           │ File │ 401 B  │ a month ago │ a month ago │ a month ago 
 4 │ youshouldeatmorecereal.txt │ File │ 768 B  │ a month ago │ a month ago │ a month ago 
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━━━━━
> ls | reject readonly accessed
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━━━━━
 # │ name                       │ type │ size   │ created     │ modified 
───┼────────────────────────────┼──────┼────────┼─────────────┼─────────────
 0 │ zeusiscrazy.txt            │ File │ 556 B  │ a month ago │ a month ago 
 1 │ coww.txt                   │ File │  24 B  │ a month ago │ a month ago 
 2 │ randomweirdstuff.txt       │ File │ 197 B  │ a month ago │ a month ago 
 3 │ abaracadabra.txt           │ File │ 401 B  │ a month ago │ a month ago 
 4 │ youshouldeatmorecereal.txt │ File │ 768 B  │ a month ago │ a month ago 
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━━━━━
```
