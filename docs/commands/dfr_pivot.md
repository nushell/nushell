---
title: dfr pivot
layout: command
version: 0.59.0
---

Performs a pivot operation on a groupby object

## Signature

```> dfr pivot (pivot-column) (value-column) (operation)```

## Parameters

 -  `pivot-column`: pivot column to perform pivot
 -  `value-column`: value column to perform pivot
 -  `operation`: aggregate operation

## Examples

Pivot a dataframe on b and aggregation on col c
```shell
[[a b c]; [one x 1] [two y 2]] | dfr to-df | dfr group-by a | dfr pivot b c sum
```

