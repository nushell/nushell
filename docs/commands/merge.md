---
title: merge
layout: command
version: 0.59.0
---

Merge a table into an input table

## Signature

```> merge (block)```

## Parameters

 -  `block`: the block to run and merge into the table

## Examples

Merge an index column into the input table
```shell
> [a b c] | wrap name | merge { [1 2 3] | wrap index }
```

Merge two records
```shell
> {a: 1, b: 2} | merge { {c: 3} }
```
