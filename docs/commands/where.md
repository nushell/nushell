# where

This command filters the content of a table based on a condition passed as a parameter, which must be a boolean expression making use of any of the table columns. Other commands such as `ls` are capable of feeding `where` with their output through pipelines.

## Usage
```shell
> [input-command] | where [condition]
```

## Examples 

```shell
> ls | where size > 4kb
━━━┯━━━━━━━━━━━━┯━━━━━━┯━━━━━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━━━━━
 # │ name       │ type │ size    │ created     │ accessed    │ modified 
───┼────────────┼──────┼─────────┼─────────────┼─────────────┼─────────────
 0 │ Cargo.lock │ File │ 87.2 KB │ 7 hours ago │ 7 hours ago │ 7 hours ago 
 1 │ README.md  │ File │ 19.5 KB │ 7 hours ago │ 7 hours ago │ 7 hours ago 
 2 │ Cargo.toml │ File │  4.7 KB │ 7 hours ago │ 7 hours ago │ 7 hours ago 
━━━┷━━━━━━━━━━━━┷━━━━━━┷━━━━━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━━━━━
```

```shell
> ps | where cpu > 0
━━━┯━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━
 # │ pid   │ name                  │ status   │ cpu 
───┼───────┼───────────────────────┼──────────┼───────────────────
 0 │  1546 │ Xorg                  │ Sleeping │ 10.65405000000000 
 1 │  1769 │ gnome-shell           │ Sleeping │ 5.271094000000000 
 2 │  2153 │ gnome-terminal-server │ Sleeping │ 5.193664000000000 
 3 │ 13556 │ nu_plugin_ps          │ Sleeping │ 40.70250000000000 
━━━┷━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━
```
