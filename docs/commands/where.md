# where

This command filters the content of a table based on a condition passed as a parameter, which must be a boolean expression making use of any of the table columns. Other commands such as `ls` are capable of feeding `where` with their output through pipelines.

Where has two general forms:
- `where <column_name> <comparison> <value>`
- `where <column_name>`

## Where with comparison

In the first form, `where` is passed a column name that the filter will run against. Next, is the operator used to compare this column to its value. The following operators are supported:

- `<` (less than)
- `<=` (less than or equal)
- `>` (greater than)
- `>=` (greater than or equal)
- `!=` (not equal)
- `==` (equal)

Strings have two additional operators:
- `=~` (fuzzy match to allow)
- `!~` (fuzzy match to not allow)

Dates can also be compared using the duration types. For example, `where accessed > 2w` will check the date in accessed to see if it's greater than 2 weeks ago. Durations currently allow these abbreviations:

- `1s` (one second)
- `1m` (one minute)
- `1h` (one hour)
- `1d` (one day)
- `1w` (one week)
- `1M` (one month)
- `1y` (one year)

## Boolean check

Where with the form `| where readonly` is used to check boolean values. For example, the command `ls --full | where readonly` will list only those files that are readonly.

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

```shell
> ls | where accessed <= 1w
━━━┯━━━━━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━
 # │ name          │ type      │ size     │ accessed   │ modified 
───┼───────────────┼───────────┼──────────┼────────────┼────────────
 0 │ Cargo.toml    │ File      │   4.7 KB │ 2 days ago │ 2 days ago 
 1 │ target        │ Directory │   4.1 KB │ 2 days ago │ 2 days ago 
 2 │ Makefile.toml │ File      │    449 B │ 4 days ago │ 4 days ago 
 3 │ README.md     │ File      │  19.5 KB │ 2 days ago │ 2 days ago 
 4 │ Cargo.lock    │ File      │ 170.7 KB │ 2 days ago │ 2 days ago 
 5 │ crates        │ Directory │   4.1 KB │ 2 days ago │ 2 days ago 
 6 │ TODO.md       │ File      │   1.3 KB │ 2 days ago │ 2 days ago 
━━━┷━━━━━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━
```

```shell
> ls | where name =~ "yml"
━━━━━━━━━━━━━┯━━━━━━┯━━━━━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━
 name        │ type │ size  │ accessed   │ modified 
─────────────┼──────┼───────┼────────────┼────────────
 .gitpod.yml │ File │ 780 B │ a week ago │ a week ago 
━━━━━━━━━━━━━┷━━━━━━┷━━━━━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━
```
