# skip-while

Skips rows while the condition matches.

## Usage

```shell
> [input-command] | skip-while <condition>
```

## Examples

If we open a file with a list of contacts, we get all of the contacts.

```shell
> open contacts.csv | sort-by "last name"
───┬────────────┬───────────┬──────────────────
 # │ first name │ last name │ email
───┼────────────┼───────────┼──────────────────
 0 │ John       │ Abbot     │ abbot@email.com
 1 │ Chris      │ Beasly    │ beasly@email.com
 2 │ Jane       │ Carver    │ carver@email.com
 3 │ Francis    │ Davis     │ davis@email.com
───┴────────────┴───────────┴──────────────────
```

To exclude skip contacts with last names starting with 'A' or 'B', use skip-while:

```shell
> open contacts.csv | sort-by "last name" |  skip-while "last name" < "C"
───┬────────────┬───────────┬──────────────────
 # │ first name │ last name │ email
───┼────────────┼───────────┼──────────────────
 0 │ Jane       │ Carver    │ carver@email.com
 1 │ Francis    │ Davis     │ davis@email.com
───┴────────────┴───────────┴──────────────────
```

Note that the order of input rows matters. Once a single row does not match the condition, all following rows are included in the output, whether or not they match the condition:

```shell
> open contacts.csv | skip-while "last name" < "C"
───┬────────────┬───────────┬──────────────────
 # │ first name │ last name │ email
───┼────────────┼───────────┼──────────────────
 0 │ Jane       │ Carver    │ carver@email.com
 1 │ Chris      │ Beasly    │ beasly@email.com
 2 │ Francis    │ Davis     │ davis@email.com
───┴────────────┴───────────┴──────────────────
```

See the `where` command to filter each individual row by a condition, regardless of order.
