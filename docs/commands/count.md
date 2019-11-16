# count

This command counts the number of rows in a table.

## Examples -

```shell
> ls
━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━━┯━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━
 #  │ name                         │ type      │ readonly │ size    │ created      │ accessed     │ modified 
────┼──────────────────────────────┼───────────┼──────────┼─────────┼──────────────┼──────────────┼──────────────
  0 │ Desktop                      │ Directory │          │  4.1 KB │ 2 months ago │ 2 months ago │ 2 months ago 
  1 │ aur                          │ Directory │          │  4.1 KB │ 4 hours ago  │ 4 hours ago  │ 4 hours ago 
...
 75 │ .emulator_console_auth_token │ File      │          │   16 B  │ 2 months ago │ 2 months ago │ 2 months ago 
 76 │ bin                          │ Directory │          │  4.1 KB │ 2 months ago │ 2 months ago │ 2 months ago 
━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━┷━━━━━━━━━┷━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━
> ls | count
━━━━━━━━━
 <value> 
─────────
      77 
━━━━━━━━━
> ls | get name | count
━━━━━━━━━
 <value> 
─────────
      77 
━━━━━━━━━
> ls | where type == File | count
━━━━━━━━━
 <value> 
─────────
      29 
━━━━━━━━━
> ls | where type == Directory | count
━━━━━━━━━
 <value> 
─────────
      48 
━━━━━━━━━
> ls | where size > 2KB | count
━━━━━━━━━
 <value> 
─────────
      57 
━━━━━━━━━
```
