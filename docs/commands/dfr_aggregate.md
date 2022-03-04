---
title: dfr aggregate
layout: command
version: 0.59.1
---

Performs an aggregation operation on a dataframe and groupby object

## Signature

```> dfr aggregate (operation_name) --quantile --explicit```

## Parameters

 -  `operation_name`:
	Dataframes: mean, sum, min, max, quantile, median, var, std
	GroupBy: mean, sum, min, max, first, last, nunique, quantile, median, var, std, count
 -  `--quantile {number}`: quantile value for quantile operation
 -  `--explicit`: returns explicit names for groupby aggregations

## Examples

Aggregate sum by grouping by column a and summing on col b
```shell
> [[a b]; [one 1] [one 2]] | dfr to-df | dfr group-by a | dfr aggregate sum
```

Aggregate sum in dataframe columns
```shell
> [[a b]; [4 1] [5 2]] | dfr to-df | dfr aggregate sum
```

Aggregate sum in series
```shell
> [4 1 5 6] | dfr to-df | dfr aggregate sum
```
