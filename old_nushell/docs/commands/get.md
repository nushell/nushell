# get

Open given cells as text.

Syntax: `get  ...args`

## Parameters

* `args`: optionally return additional data by path

## Examples

If we run `sys` we receive a table which contains tables itself:

```shell
> sys
─────────┬─────────────────────────────────────────
 host    │ [row 7 columns]
 cpu     │ [row cores current ghz max ghz min ghz]
 disks   │ [table 4 rows]
 mem     │ [row free swap free swap total total]
 net     │ [table 19 rows]
 battery │ [table 1 rows]
─────────┴─────────────────────────────────────────
```

To access one of the embedded tables we can use the `get` command

```shell
> sys | get cpu
─────────────┬────────
 cores       │ 16
 current ghz │ 2.4000
 min ghz     │ 2.4000
 max ghz     │ 2.4000
─────────────┴────────
```

```shell
> sys | get battery
───────────────┬──────────
 vendor        │ DSY
 model         │ bq40z651
 cycles        │ 43
 mins to empty │ 70.0000
───────────────┴──────────
```

There's also the ability to pass multiple parameters to `get` which results in an output like this

```shell
sys | get cpu battery
───┬───────┬─────────────┬─────────┬─────────
 # │ cores │ current ghz │ min ghz │ max ghz
───┼───────┼─────────────┼─────────┼─────────
 0 │    16 │      2.4000 │  2.4000 │  2.4000
───┴───────┴─────────────┴─────────┴─────────
───┬────────┬──────────┬────────┬───────────────
 # │ vendor │ model    │ cycles │ mins to empty
───┼────────┼──────────┼────────┼───────────────
 1 │ DSY    │ bq40z651 │     43 │       70.0000
───┴────────┴──────────┴────────┴───────────────
```
