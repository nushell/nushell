---
title: flatten
layout: command
version: 0.59.0
---

Flatten the table.

## Signature

```> flatten ...rest```

## Parameters

 -  `...rest`: optionally flatten data by column

## Examples

flatten a table
```shell
> [[N, u, s, h, e, l, l]] | flatten
```

flatten a table, get the first item
```shell
> [[N, u, s, h, e, l, l]] | flatten | first
```

flatten a column having a nested table
```shell
> [[origin, people]; [Ecuador, ([[name, meal]; ['Andres', 'arepa']])]] | flatten | get meal
```

restrict the flattening by passing column names
```shell
> [[origin, crate, versions]; [World, ([[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten versions | last | get versions
```

Flatten inner table
```shell
> { a: b, d: [ 1 2 3 4 ],  e: [ 4 3  ] } | flatten
```
