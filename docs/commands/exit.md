# exit

Exits the nu shell. If you have multiple nu shells, use `exit --now` to exit all of them.

## Examples 

```shell
> exit
```

```
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path 
───┼───┼────────────┼─────────────────────────────────────
 0 │   │ filesystem │ /home/jonathanturner/Source/nushell 
 1 │   │ filesystem │ /home 
 2 │ X │ filesystem │ /usr 
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> exit
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path 
───┼───┼────────────┼─────────────────────────────────────
 0 │   │ filesystem │ /home/jonathanturner/Source/nushell 
 1 │ X │ filesystem │ /home 
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
> exit --now
exits both the shells
```
