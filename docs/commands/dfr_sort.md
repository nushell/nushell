---
title: dfr sort
layout: command
version: 0.59.0
---

Creates new sorted dataframe or series

## Signature

```> dfr sort ...rest --reverse```

## Parameters

 -  `...rest`: column names to sort dataframe
 -  `--reverse`: invert sort

## Examples

Create new sorted dataframe
```shell
> [[a b]; [3 4] [1 2]] | dfr to-df | dfr sort a
```

Create new sorted series
```shell
> [3 4 1 2] | dfr to-df | dfr sort
```
