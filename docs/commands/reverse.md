# reverse

This command reverses the order of the elements in a sorted table. 

## Examples 

```shell
> ls | sort-by name
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━
 # │ name                       │ type │ readonly │ size   │ accessed       │ modified 
───┼────────────────────────────┼──────┼──────────┼────────┼────────────────┼────────────────
 0 │ abaracadabra.txt           │ File │          │ 401 B  │ 23 minutes ago │ 16 minutes ago 
 1 │ coww.txt                   │ File │          │  24 B  │ 22 minutes ago │ 17 minutes ago 
 2 │ randomweirdstuff.txt       │ File │          │ 197 B  │ 21 minutes ago │ 18 minutes ago 
 3 │ youshouldeatmorecereal.txt │ File │          │ 768 B  │ 30 seconds ago │ now 
 4 │ zeusiscrazy.txt            │ File │          │ 556 B  │ 22 minutes ago │ 18 minutes ago 
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━
> ls | sort-by name | reverse
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━
 # │ name                       │ type │ readonly │ size   │ accessed       │ modified 
───┼────────────────────────────┼──────┼──────────┼────────┼────────────────┼────────────────
 0 │ zeusiscrazy.txt            │ File │          │ 556 B  │ 22 minutes ago │ 19 minutes ago 
 1 │ youshouldeatmorecereal.txt │ File │          │ 768 B  │ 39 seconds ago │ 18 seconds ago 
 2 │ randomweirdstuff.txt       │ File │          │ 197 B  │ 21 minutes ago │ 18 minutes ago 
 3 │ coww.txt                   │ File │          │  24 B  │ 22 minutes ago │ 18 minutes ago 
 4 │ abaracadabra.txt           │ File │          │ 401 B  │ 23 minutes ago │ 16 minutes ago 
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━
```

```shell
> ls | sort-by size
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━
 # │ name                       │ type │ readonly │ size   │ accessed       │ modified 
───┼────────────────────────────┼──────┼──────────┼────────┼────────────────┼────────────────
 0 │ coww.txt                   │ File │          │  24 B  │ 22 minutes ago │ 18 minutes ago 
 1 │ randomweirdstuff.txt       │ File │          │ 197 B  │ 21 minutes ago │ 18 minutes ago 
 2 │ abaracadabra.txt           │ File │          │ 401 B  │ 23 minutes ago │ 16 minutes ago 
 3 │ zeusiscrazy.txt            │ File │          │ 556 B  │ 22 minutes ago │ 19 minutes ago 
 4 │ youshouldeatmorecereal.txt │ File │          │ 768 B  │ a minute ago   │ 26 seconds ago 
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━
> ls | sort-by size | reverse
━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━
 # │ name                       │ type │ readonly │ size   │ accessed       │ modified 
───┼────────────────────────────┼──────┼──────────┼────────┼────────────────┼────────────────
 0 │ youshouldeatmorecereal.txt │ File │          │ 768 B  │ a minute ago   │ 32 seconds ago 
 1 │ zeusiscrazy.txt            │ File │          │ 556 B  │ 22 minutes ago │ 19 minutes ago 
 2 │ abaracadabra.txt           │ File │          │ 401 B  │ 23 minutes ago │ 16 minutes ago 
 3 │ randomweirdstuff.txt       │ File │          │ 197 B  │ 21 minutes ago │ 18 minutes ago 
 4 │ coww.txt                   │ File │          │  24 B  │ 22 minutes ago │ 18 minutes ago 
━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━
```
