---
title: dfr group-by
layout: command
version: 0.59.0
---

Creates a groupby object that can be used for other aggregations

## Signature

```> dfr group-by ...rest```

## Parameters

 -  `...rest`: groupby columns

## Examples

Grouping by column a
```shell
> [[a b]; [one 1] [one 2]] | dfr to-df | dfr group-by a
```
