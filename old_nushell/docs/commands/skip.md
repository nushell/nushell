# skip

Skips the first 'n' rows of a table.

## Usage

```shell
> [input-command] | skip (n)
```

## Examples

If we open a file with a list of contacts, we get all of the contacts.

```shell
> open contacts.csv
───┬─────────┬──────┬─────────────────
 # │ first   │ last │ email
───┼─────────┼──────┼─────────────────
 0 │ John    │ Doe  │ doe.1@email.com
 1 │ Jane    │ Doe  │ doe.2@email.com
 2 │ Chris   │ Doe  │ doe.3@email.com
 3 │ Francis │ Doe  │ doe.4@email.com
───┴─────────┴──────┴─────────────────
```

To ignore the first 2 contacts, we can `skip` them.

```shell
> open contacts.csv | skip 2
───┬─────────┬──────┬─────────────────
 # │ first   │ last │ email
───┼─────────┼──────┼─────────────────
 0 │ Chris   │ Doe  │ doe.3@email.com
 1 │ Francis │ Doe  │ doe.4@email.com
───┴─────────┴──────┴─────────────────
```
