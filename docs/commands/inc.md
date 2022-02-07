# inc

This command increments the value of variable by one.

## Examples

```shell
> open rustfmt.toml
─────────┬──────
 edition │ 2018
─────────┴──────
```

```shell
> open rustfmt.toml | inc edition
─────────┬──────
 edition │ 2019
─────────┴──────
```

```shell
> open Cargo.toml | get package.version
0.15.1
```

```shell
> open Cargo.toml | inc package.version --major | get package.version
1.0.0
```

```shell
> open Cargo.toml | inc package.version --minor | get package.version
0.16.0
```

```shell
> open Cargo.toml | inc package.version --patch | get package.version
0.15.2
```
