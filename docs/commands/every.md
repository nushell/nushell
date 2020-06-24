# every

Selects every n-th row of a table, starting from the first one. With the `--skip` flag, every n-th row will be skipped, inverting the original functionality.

Syntax: `> [input-command] | every <stride> {flags}`

## Flags

* `--skip`, `-s`: Skip the rows that would be returned, instead of selecting them

## Examples

```shell
> open contacts.csv
───┬─────────┬──────┬─────────────────
 # │ first   │ last │ email
───┼─────────┼──────┼─────────────────
 0 │ John    │ Doe  │ doe.1@email.com
 1 │ Jane    │ Doe  │ doe.2@email.com
 2 │ Chris   │ Doe  │ doe.3@email.com
 3 │ Francis │ Doe  │ doe.4@email.com
 4 │ Stella  │ Doe  │ doe.5@email.com
───┴─────────┴──────┴─────────────────
```

```shell
> open contacts.csv | every 2
───┬─────────┬──────┬─────────────────
 # │ first   │ last │ email
───┼─────────┼──────┼─────────────────
 0 │ John    │ Doe  │ doe.1@email.com
 2 │ Chris   │ Doe  │ doe.3@email.com
 4 │ Stella  │ Doe  │ doe.5@email.com
───┴─────────┴──────┴─────────────────
```

```shell
> open contacts.csv | every 2 --skip
───┬─────────┬──────┬─────────────────
 # │ first   │ last │ email
───┼─────────┼──────┼─────────────────
 1 │ Jane    │ Doe  │ doe.2@email.com
 3 │ Francis │ Doe  │ doe.4@email.com
───┴─────────┴──────┴─────────────────
```
