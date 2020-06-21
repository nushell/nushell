# shells

Lists all the active nu shells with a number/index, a name and the path. Also marks the current nu shell.

## Examples

```shell
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼─────────────────────────────────────
 0 │   │ filesystem │ /home/jonathanturner/Source/nushell
 1 │   │ filesystem │ /usr
 2 │ X │ filesystem │ /home
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
/> shells
━━━┯━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name                                             │ path
───┼───┼──────────────────────────────────────────────────┼─────────────────────────────────────
 0 │   │ filesystem                                       │ /home/jonathanturner/Source/nushell
 1 │ X │ {/home/jonathanturner/Source/nushell/Cargo.toml} │ /
━━━┷━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```
