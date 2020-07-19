
# sort-by

The `sort-by` command sorts the table being displayed in the terminal by a chosen column(s).

`sort-by` takes multiple arguments (being the names of columns) sorting by each argument in order.

## Flags

* `-i`, `--insensitive`: Sort string-based columns case insensitively

## Examples

```shell
> ls | sort-by size
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
> ls | sort-by size name
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

```shell
> ls | sort-by accessed
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

Within the Nushell repository...

```shell
> ls | sort-by --insensitive name
────┬────────────────────┬──────┬──────────┬──────────────
 #  │ name               │ type │ size     │ modified
────┼────────────────────┼──────┼──────────┼──────────────
  0 │ assets             │ Dir  │    128 B │ 6 months ago
  1 │ build.rs           │ File │     78 B │ 5 months ago
  2 │ Cargo.lock         │ File │ 118.3 KB │ 1 hour ago
  3 │ Cargo.toml         │ File │   5.5 KB │ 1 hour ago
  4 │ CODE_OF_CONDUCT.md │ File │   3.4 KB │ 1 hour ago
  5 │ CONTRIBUTING.md    │ File │   1.3 KB │ 1 hour ago
  6 │ crates             │ Dir  │    832 B │ 1 hour ago
  7 │ debian             │ Dir  │    352 B │ 6 months ago
  8 │ docker             │ Dir  │    288 B │ 4 months ago
  9 │ docs               │ Dir  │    192 B │ 1 hour ago
 10 │ features.toml      │ File │    632 B │ 5 months ago
 11 │ images             │ Dir  │    160 B │ 6 months ago
 12 │ LICENSE            │ File │   1.1 KB │ 4 months ago
 13 │ Makefile.toml      │ File │    449 B │ 6 months ago
 14 │ README.build.txt   │ File │    192 B │ 1 hour ago
 15 │ README.md          │ File │  16.0 KB │ 1 hour ago
 16 │ rustfmt.toml       │ File │     16 B │ 6 months ago
 17 │ src                │ Dir  │    128 B │ 1 week ago
 18 │ target             │ Dir  │    160 B │ 1 day ago
 19 │ tests              │ Dir  │    192 B │ 4 months ago
 20 │ TODO.md            │ File │      0 B │ 1 week ago
 21 │ wix                │ Dir  │    128 B │ 1 hour ago
────┴────────────────────┴──────┴──────────┴──────────────
```

Within the Nushell repository...

```shell
> ls | sort-by --insensitive type name
────┬────────────────────┬──────┬──────────┬──────────────
 #  │ name               │ type │ size     │ modified
────┼────────────────────┼──────┼──────────┼──────────────
  0 │ assets             │ Dir  │    128 B │ 6 months ago
  1 │ crates             │ Dir  │    832 B │ 1 hour ago
  2 │ debian             │ Dir  │    352 B │ 6 months ago
  3 │ docker             │ Dir  │    288 B │ 4 months ago
  4 │ docs               │ Dir  │    192 B │ 1 hour ago
  5 │ images             │ Dir  │    160 B │ 6 months ago
  6 │ src                │ Dir  │    128 B │ 1 week ago
  7 │ target             │ Dir  │    160 B │ 1 day ago
  8 │ tests              │ Dir  │    192 B │ 4 months ago
  9 │ wix                │ Dir  │    128 B │ 1 hour ago
 10 │ build.rs           │ File │     78 B │ 5 months ago
 11 │ Cargo.lock         │ File │ 118.3 KB │ 1 hour ago
 12 │ Cargo.toml         │ File │   5.5 KB │ 1 hour ago
 13 │ CODE_OF_CONDUCT.md │ File │   3.4 KB │ 1 hour ago
 14 │ CONTRIBUTING.md    │ File │   1.3 KB │ 1 hour ago
 15 │ features.toml      │ File │    632 B │ 5 months ago
 16 │ LICENSE            │ File │   1.1 KB │ 4 months ago
 17 │ Makefile.toml      │ File │    449 B │ 6 months ago
 18 │ README.build.txt   │ File │    192 B │ 1 hour ago
 19 │ README.md          │ File │  16.0 KB │ 1 hour ago
 20 │ rustfmt.toml       │ File │     16 B │ 6 months ago
 21 │ TODO.md            │ File │      0 B │ 1 week ago
────┴────────────────────┴──────┴──────────┴──────────────
```
