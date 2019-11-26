
# sort-by

The `sort-by` command sorts the table being displayed in the terminal by a chosen column(s). 

`sort-by` takes multiple arguments (being the names of columns) sorting by each argument in order. 


## Examples -

```shell 
/home/example> ls | sort-by size
━━━┯━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━
 # │ name │ type │ readonly │ size   │ accessed       │ modified 
───┼──────┼──────┼──────────┼────────┼────────────────┼────────────────
 0 │ az   │ File │          │  18 B  │ 4 minutes ago  │ 4 minutes ago 
 1 │ a    │ File │          │  18 B  │ 4 minutes ago  │ 38 minutes ago 
 2 │ ad   │ File │          │  18 B  │ 4 minutes ago  │ 4 minutes ago 
 3 │ ac   │ File │          │  18 B  │ 4 minutes ago  │ 4 minutes ago 
 4 │ ab   │ File │          │  18 B  │ 4 minutes ago  │ 4 minutes ago 
 5 │ c    │ File │          │ 102 B  │ 35 minutes ago │ 35 minutes ago 
 6 │ d    │ File │          │ 189 B  │ 35 minutes ago │ 34 minutes ago 
 7 │ b    │ File │          │ 349 B  │ 35 minutes ago │ 35 minutes ago 
━━━┷━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━
```

```shell 
/home/example> ls | sort-by size name
━━━┯━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━
 # │ name │ type │ readonly │ size   │ accessed       │ modified 
───┼──────┼──────┼──────────┼────────┼────────────────┼────────────────
 0 │ a    │ File │          │  18 B  │ 4 minutes ago  │ 39 minutes ago 
 1 │ ab   │ File │          │  18 B  │ 4 minutes ago  │ 4 minutes ago 
 2 │ ac   │ File │          │  18 B  │ 4 minutes ago  │ 4 minutes ago 
 3 │ ad   │ File │          │  18 B  │ 4 minutes ago  │ 4 minutes ago 
 4 │ az   │ File │          │  18 B  │ 4 minutes ago  │ 4 minutes ago 
 5 │ c    │ File │          │ 102 B  │ 36 minutes ago │ 35 minutes ago 
 6 │ d    │ File │          │ 189 B  │ 35 minutes ago │ 35 minutes ago 
 7 │ b    │ File │          │ 349 B  │ 36 minutes ago │ 36 minutes ago 
```

```
/home/example> ls | sort-by accessed
━━━┯━━━━━━┯━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━
 # │ name │ type │ readonly │ size   │ accessed       │ modified 
───┼──────┼──────┼──────────┼────────┼────────────────┼────────────────
 0 │ b    │ File │          │ 349 B  │ 37 minutes ago │ 37 minutes ago 
 1 │ c    │ File │          │ 102 B  │ 37 minutes ago │ 37 minutes ago 
 2 │ d    │ File │          │ 189 B  │ 37 minutes ago │ 36 minutes ago 
 3 │ a    │ File │          │  18 B  │ 6 minutes ago  │ 40 minutes ago 
 4 │ ab   │ File │          │  18 B  │ 6 minutes ago  │ 6 minutes ago 
 5 │ ac   │ File │          │  18 B  │ 6 minutes ago  │ 6 minutes ago 
 6 │ ad   │ File │          │  18 B  │ 5 minutes ago  │ 5 minutes ago 
 7 │ az   │ File │          │  18 B  │ 5 minutes ago  │ 5 minutes ago 
━━━┷━━━━━━┷━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━
```