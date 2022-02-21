---
title: dfr to-parquet
layout: command
version: 0.59.0
---

Saves dataframe to parquet file

## Signature

```> dfr to-parquet (file)```

## Parameters

 -  `file`: file path to save dataframe

## Examples

Saves dataframe to csv file
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr to-parquet test.parquet
```
