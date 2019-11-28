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
----+----------------+------+----------+----------+----------------+----------------
 #  | name           | type | readonly | size     | accessed       | modified 
----+----------------+------+----------+----------+----------------+----------------
 0  | IMG_1291.jpg   | File |          | 115.5 KB | a month ago    | 4 months ago 
 1  | README.md      | File |          | 11.1 KB  | 2 days ago     | 2 days ago 
 2  | IMG_1291.png   | File |          | 589.0 KB | a month ago    | a month ago 
 3  | IMG_1381.jpg   | File |          | 81.0 KB  | a month ago    | 4 months ago 
 4  | butterfly.jpeg | File |          | 4.2 KB   | a month ago    | a month ago 
 5  | Cargo.lock     | File |          | 199.6 KB | 22 minutes ago | 22 minutes ago
```

```shell
> ps | where cpu > 10
---+-------+----------+-------+-----------------------------
 # | pid   | status   | cpu   | name 
---+-------+----------+-------+-----------------------------
 0 | 1992  | Sleeping | 44.52 | /usr/bin/gnome-shell 
 1 | 1069  | Sleeping | 16.15 |  
 2 | 24116 | Sleeping | 13.70 | /opt/google/chrome/chrome 
 3 | 21976 | Sleeping | 12.67 | /usr/share/discord/Discord
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
