---
title: dfr slice
layout: command
version: 0.59.0
---

Creates new dataframe from a slice of rows

## Signature

```> dfr slice (offset) (size)```

## Parameters

 -  `offset`: start of slice
 -  `size`: size of slice

## Examples

Create new dataframe from a slice of the rows
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr slice 0 1
```
