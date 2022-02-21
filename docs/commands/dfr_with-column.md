---
title: dfr with-column
layout: command
version: 0.59.0
---

Adds a series to the dataframe

## Signature

```> dfr with-column (series) --name```

## Parameters

 -  `series`: series to be added
 -  `--name {string}`: column name

## Examples

Adds a series to the dataframe
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr with-column ([5 6] | dfr to-df) --name c
```
