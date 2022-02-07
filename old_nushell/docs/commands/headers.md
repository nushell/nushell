# headers

Use `headers` to turn the first row of a table into meaningful column names.

As demonstrated in the following example, it's particularly handy when working with spreadsheets.

## Examples

```shell
> open sample_data.ods | get SalesOrders
────┬────────────┬─────────┬──────────┬─────────┬─────────┬───────────┬───────────
 #  │  Column0   │ Column1 │ Column2  │ Column3 │ Column4 │  Column5  │  Column6
────┼────────────┼─────────┼──────────┼─────────┼─────────┼───────────┼───────────
  0 │ OrderDate  │ Region  │ Rep      │ Item    │ Units   │ Unit Cost │ Total
  1 │ 2018-01-06 │ East    │ Jones    │ Pencil  │ 95.0000 │    1.9900 │  189.0500
```

```shell
> open sample_data.ods | get SalesOrders | headers
────┬────────────┬─────────┬──────────┬─────────┬─────────┬───────────┬───────────
 #  │ OrderDate  │ Region  │   Rep    │  Item   │  Units  │ Unit Cost │   Total
────┼────────────┼─────────┼──────────┼─────────┼─────────┼───────────┼───────────
  0 │ 2018-01-06 │ East    │ Jones    │ Pencil  │ 95.0000 │    1.9900 │  189.0500
  1 │ 2018-01-23 │ Central │ Kivell   │ Binder  │ 50.0000 │   19.9900 │  999.4999
```
