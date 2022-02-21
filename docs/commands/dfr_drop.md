---
title: dfr drop
layout: command
version: 0.59.0
---

Creates a new dataframe by dropping the selected columns

## Signature

```> dfr drop ...rest```

## Parameters

 -  `...rest`: column names to be dropped

## Examples

drop column a
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr drop a
```
