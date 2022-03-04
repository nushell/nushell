---
title: dfr filter-with
layout: command
version: 0.59.1
---

Filters dataframe using a mask as reference

## Signature

```> dfr filter-with (mask)```

## Parameters

 -  `mask`: boolean mask used to filter data

## Examples

Filter dataframe using a bool mask
```shell
> let mask = ([true false] | dfr to-df);
    [[a b]; [1 2] [3 4]] | dfr to-df | dfr filter-with $mask
```
