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

Dates can also be compared using the duration types. For example, `where accessed > 2wk` will check the date in accessed to see if it's greater than 2 weeks ago. Durations currently allow these abbreviations:

- `1sec` (one second)
- `1min` (one minute)
- `1hr` (one hour)
- `1day` (one day)
- `1wk` (one week)

## Boolean check

Where with the form `| where readonly` is used to check boolean values. For example, the command `ls --long | where readonly` will list only those files that are readonly.

## Usage

```shell
> [input-command] | where [condition]
```

## Examples

```shell
> ls | where size > 4kb
───┬────────────┬──────┬──────────┬─────────────
 # │ name       │ type │ size     │ modified
───┼────────────┼──────┼──────────┼─────────────
 0 │ Cargo.lock │ File │ 113.3 KB │ 53 mins ago
 1 │ Cargo.toml │ File │   4.6 KB │ 53 mins ago
 2 │ README.md  │ File │  15.8 KB │ 2 mins ago
───┴────────────┴──────┴──────────┴─────────────
```

```shell
> ps | where cpu > 0
───┬───────┬──────────────────┬─────────┬────────┬──────────┬─────────
 # │ pid   │ name             │ status  │ cpu    │ mem      │ virtual
───┼───────┼──────────────────┼─────────┼────────┼──────────┼─────────
 0 │ 17917 │ nu_plugin_core_p │ Running │ 4.1678 │   2.1 MB │  4.8 GB
 1 │ 14717 │ Discord Helper ( │ Running │ 1.6842 │ 371.9 MB │  8.0 GB
 2 │ 14713 │ Discord Helper   │ Running │ 0.2099 │  27.8 MB │  5.8 GB
 3 │ 14710 │ Discord          │ Running │ 0.0883 │ 105.4 MB │  7.0 GB
 4 │  9643 │ Terminal         │ Running │ 4.0313 │ 266.4 MB │  7.6 GB
 5 │  7864 │ Microsoft.Python │ Running │ 0.9828 │ 340.9 MB │  8.0 GB
 6 │ 24402 │ Code Helper (Ren │ Running │ 1.0644 │ 337.3 MB │  8.4 GB
 7 │ 24401 │ Code Helper (Ren │ Running │ 1.0031 │ 593.5 MB │  8.6 GB
 8 │   519 │ EmojiFunctionRow │ Running │ 0.2063 │  52.7 MB │  7.5 GB
 9 │   376 │ CommCenter       │ Running │ 0.1620 │  30.0 MB │  6.5 GB
───┴───────┴──────────────────┴─────────┴────────┴──────────┴─────────

```

```shell
> ls -l | where accessed <= 1wk
───┬────────────────────┬──────┬────────┬──────────┬───────────┬─────────────┬───────┬──────────┬──────────────┬─────────────┬─────────────
 # │ name               │ type │ target │ readonly │ mode      │ uid         │ group │ size     │ created      │ accessed    │ modified
───┼────────────────────┼──────┼────────┼──────────┼───────────┼─────────────┼───────┼──────────┼──────────────┼─────────────┼─────────────
 0 │ CODE_OF_CONDUCT.md │ File │        │ No       │ rw-r--r-- │ josephlyons │ staff │   3.4 KB │ 52 mins ago  │ 52 secs ago │ 52 mins ago
 1 │ CONTRIBUTING.md    │ File │        │ No       │ rw-r--r-- │ josephlyons │ staff │   1.3 KB │ 52 mins ago  │ 4 mins ago  │ 4 mins ago
 2 │ Cargo.lock         │ File │        │ No       │ rw-r--r-- │ josephlyons │ staff │ 113.3 KB │ 52 mins ago  │ 52 mins ago │ 52 mins ago
 3 │ Cargo.toml         │ File │        │ No       │ rw-r--r-- │ josephlyons │ staff │   4.6 KB │ 52 mins ago  │ 52 mins ago │ 52 mins ago
 4 │ README.md          │ File │        │ No       │ rw-r--r-- │ josephlyons │ staff │  15.8 KB │ 52 mins ago  │ 1 min ago   │ 1 min ago
 5 │ TODO.md            │ File │        │ No       │ rw-r--r-- │ josephlyons │ staff │      0 B │ 52 mins ago  │ 52 mins ago │ 52 mins ago
 6 │ crates             │ Dir  │        │ No       │ rwxr-xr-x │ josephlyons │ staff │    704 B │ 4 months ago │ 52 mins ago │ 52 mins ago
 7 │ docs               │ Dir  │        │ No       │ rwxr-xr-x │ josephlyons │ staff │    192 B │ 5 months ago │ 52 mins ago │ 52 mins ago
 8 │ src                │ Dir  │        │ No       │ rwxr-xr-x │ josephlyons │ staff │    128 B │ 5 months ago │ 1 day ago   │ 1 day ago
 9 │ target             │ Dir  │        │ No       │ rwxr-xr-x │ josephlyons │ staff │    160 B │ 5 days ago   │ 5 days ago  │ 5 days ago
───┴────────────────────┴──────┴────────┴──────────┴───────────┴─────────────┴───────┴──────────┴──────────────┴─────────────┴─────────────
```

```shell
> ls -a | where name =~ "yml"
──────────┬─────────────
 name     │ .gitpod.yml
 type     │ File
 size     │ 866 B
 modified │ 1 month ago
──────────┴─────────────
```
