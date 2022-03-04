---
title: transpose
layout: command
version: 0.59.1
---

Transposes the table contents so rows become columns and columns become rows.

## Signature

```> transpose ...rest --header-row --ignore-titles```

## Parameters

 -  `...rest`: the names to give columns once transposed
 -  `--header-row`: treat the first row as column names
 -  `--ignore-titles`: don't transpose the column names into values

## Examples

Transposes the table contents with default column names
```shell
> echo [[c1 c2]; [1 2]] | transpose
```

Transposes the table contents with specified column names
```shell
> echo [[c1 c2]; [1 2]] | transpose key val
```

Transposes the table without column names and specify a new column name
```shell
> echo [[c1 c2]; [1 2]] | transpose -i val
```
