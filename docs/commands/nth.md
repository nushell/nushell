# nth

This command returns the nth row of a table, starting from 0.  
If the number given is less than 0 or more than the number of rows, nothing is returned.

### Usage
```shell
> [input-command] | nth <row number>  ...args
```
### Parameters:
* `<row number>` the number of the row to return
* `args`: Optionally return more rows

## Examples
```shell
> ls
━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━
 # │ name       │ type      │ readonly │ size   │ accessed      │ modified 
───┼────────────┼───────────┼──────────┼────────┼───────────────┼───────────────
 0 │ Cargo.toml │ File      │          │ 239 B  │ 2 minutes ago │ 2 minutes ago 
 1 │ .git       │ Directory │          │ 4.1 KB │ 2 minutes ago │ 2 minutes ago 
 2 │ .gitignore │ File      │          │  19 B  │ 2 minutes ago │ 2 minutes ago 
 3 │ src        │ Directory │          │ 4.1 KB │ 2 minutes ago │ 2 minutes ago 
━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━

> ls | nth 0
━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━
 # │ name       │ type      │ readonly │ size   │ accessed      │ modified 
───┼────────────┼───────────┼──────────┼────────┼───────────────┼───────────────
 0 │ Cargo.toml │ File      │          │ 239 B  │ 2 minutes ago │ 2 minutes ago 
━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━

> ls | nth 0 2
━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━┯━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━
 # │ name       │ type      │ readonly │ size   │ accessed      │ modified 
───┼────────────┼───────────┼──────────┼────────┼───────────────┼───────────────
 0 │ Cargo.toml │ File      │          │ 239 B  │ 2 minutes ago │ 2 minutes ago 
 2 │ .gitignore │ File      │          │  19 B  │ 2 minutes ago │ 2 minutes ago 
━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━┷━━━━━━━━┷━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━

> ls | nth 5
```