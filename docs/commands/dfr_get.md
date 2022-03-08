---
title: dfr get
layout: command
version: 0.59.1
---

Creates dataframe with the selected columns

## Signature

```> dfr get ...rest```

## Parameters

 -  `...rest`: column names to sort dataframe

## Examples

Creates dataframe with selected columns
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr get a
```
