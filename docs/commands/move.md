---
title: move
layout: command
version: 0.59.1
---

Move columns before or after other columns

## Signature

```> move ...columns --after --before```

## Parameters

 -  `...columns`: the columns to move
 -  `--after {string}`: the column that will precede the columns moved
 -  `--before {string}`: the column that will be the next after the columns moved

## Examples

Move a column before the first column
```shell
> [[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move index --before name
```

Move multiple columns after the last column and reorder them
```shell
> [[name value index]; [foo a 1] [bar b 2] [baz c 3]] | move value name --after index
```

Move columns of a record
```shell
> { name: foo, value: a, index: 1 } | move name --before index
```
