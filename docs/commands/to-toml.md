# to toml

Converts table data into toml text.

## Example

```shell
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────
 0 │ X │ filesystem │ /home/shaurya
 1 │   │ filesystem │ /home/shaurya/Pictures
 2 │   │ filesystem │ /home/shaurya/Desktop
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
> shells | to toml
[[]]
" " = "X"
name = "filesystem"
path = "/home/shaurya"

[[]]
" " = " "
name = "filesystem"
path = "/home/shaurya/Pictures"

[[]]
" " = " "
name = "filesystem"
path = "/home/shaurya/Desktop"
```

```shell
> open cargo_sample.toml
━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━
 dependencies   │ dev-dependencies │ package
────────────────┼──────────────────┼────────────────
 [table: 1 row] │ [table: 1 row]   │ [table: 1 row]
━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━
```

```shell
> open cargo_sample.toml | to toml
[dependencies]
ansi_term = "0.11.0"
directories = "2.0.2"
byte-unit = "2.1.0"
bytes = "0.4.12"
chrono-humanize = "0.0.11"
chrono-tz = "0.5.1"
clap = "2.33.0"
conch-parser = "0.1.1"
derive-new = "0.5.6"
dunce = "1.0.0"
futures-sink-preview = "0.3.0-alpha.16"
futures_codec = "0.2.2"
getset = "0.0.7"
itertools = "0.8.0"
lalrpop-util = "0.17.0"
language-reporting = "0.3.0"
log = "0.4.6"
logos = "0.10.0-rc2"
logos-derive = "0.10.0-rc2"
nom = "5.0.0-beta1"
ordered-float = "1.0.2"
pretty_env_logger = "0.3.0"
prettyprint = "0.6.0"
prettytable-rs = "0.8.0"
regex = "1.1.6"
rustyline = "4.1.0"
serde = "1.0.91"
serde_derive = "1.0.91"
serde_json = "1.0.39"
sysinfo = "0.8.4"
term = "0.5.2"
tokio-fs = "0.1.6"
toml = "0.5.1"
toml-query = "0.9.0"

[dependencies.chrono]
features = ["serde"]
version = "0.4.6"

[dependencies.cursive]
default-features = false
features = ["pancurses-backend"]
version = "0.26.0"

[dependencies.futures-preview]
features = ["compat", "io-compat"]
version = "0.3.0-alpha.16"

[dependencies.indexmap]
features = ["serde-1"]
version = "1.0.2"

[dependencies.pancurses]
features = ["win32a"]
version = "0.16"

[dev-dependencies]
pretty_assertions = "0.6.1"

[package]
authors = ["The Nu Project Contributors"]
description = "A shell for the GitHub era"
edition = "2018"
license = "ISC"
name = "nu"
version = "0.1.1"
```
