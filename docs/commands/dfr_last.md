---
title: dfr last
layout: command
version: 0.59.1
---

Creates new dataframe with tail rows

## Signature

```> dfr last (rows)```

## Parameters

 -  `rows`: Number of rows for tail

## Examples

Create new dataframe with last rows
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr last 1
```
