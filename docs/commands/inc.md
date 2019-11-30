# inc

This command increments the value of variable by one.

## Examples

```shell
> open rustfmt.toml
━━━━━━━━━
 edition 
─────────
 2018 
━━━━━━━━━
> open rustfmt.toml | inc edition
━━━━━━━━━
 edition 
─────────
 2019 
━━━━━━━━━
```

```shell
> open Cargo.toml | get package.version
0.1.3
> open Cargo.toml | inc package.version --major | get package.version
1.0.0
> open Cargo.toml | inc package.version --minor | get package.version
0.2.0
> open Cargo.toml | inc package.version --patch | get package.version
0.1.4
```