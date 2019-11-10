# pick

This command displays only the column names passed on to it.

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
> ls | pick name 
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │ name 
───┼────────────────────────────
 0 │ zeusiscrazy.txt 
 1 │ coww.txt 
 2 │ randomweirdstuff.txt 
 3 │ abaracadabra.txt 
 4 │ youshouldeatmorecereal.txt 
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

The order in which you put the column names matters: 

```shell
> ls | pick type name size
━━━┯━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━
 # │ type │ name                       │ size 
───┼──────┼────────────────────────────┼────────
 0 │ File │ zeusiscrazy.txt            │ 556 B  
 1 │ File │ coww.txt                   │  24 B  
 2 │ File │ randomweirdstuff.txt       │ 197 B  
 3 │ File │ abaracadabra.txt           │ 401 B  
 4 │ File │ youshouldeatmorecereal.txt │ 768 B  
━━━┷━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━
> ls | pick size type name
━━━┯━━━━━━━━┯━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │ size   │ type │ name 
───┼────────┼──────┼────────────────────────────
 0 │ 556 B  │ File │ zeusiscrazy.txt 
 1 │  24 B  │ File │ coww.txt 
 2 │ 197 B  │ File │ randomweirdstuff.txt 
 3 │ 401 B  │ File │ abaracadabra.txt 
 4 │ 768 B  │ File │ youshouldeatmorecereal.txt 
━━━┷━━━━━━━━┷━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```
