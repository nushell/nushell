# wrap

Wraps data in a table

Syntax: `wrap <column>`

## Parameters

- `column`: the (optional) name of the column the data should be stored in.

## Examples

`wrap` will give a name to a column of `<value>` data:

```shell
/home/chris> ls | get name
───┬──────────────
 # │ <value>
───┼──────────────
 0 │ americas.csv
 1 │ iso.csv
───┴──────────────

/home/chris> ls | get name | wrap filename
───┬──────────────
 # │ filename
───┼──────────────
 0 │ americas.csv
 1 │ iso.csv
───┴──────────────
```

`wrap` will encapsulate rows as embedded tables :

```shell
/home/chris> ls | select name type size
───┬──────────────┬──────┬─────────
 # │ name         │ type │ size
───┼──────────────┼──────┼─────────
 0 │ americas.csv │ File │   317 B
 1 │ iso.csv      │ File │ 20.8 KB
───┴──────────────┴──────┴─────────

/home/chris> ls | select name type size | each {wrap details}
───┬────────────────
 # │ details
───┼────────────────
 0 │ [table 1 rows]
 1 │ [table 1 rows]
───┴────────────────
```

`wrap` will encapsulate a whole table as an embedded table :

```shell
/home/chris> ls | wrap files
───────┬────────────────
 files │ [table 2 rows]
───────┴────────────────
```

