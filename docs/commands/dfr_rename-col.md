---
title: dfr rename-col
layout: command
version: 0.59.1
---

rename a dataframe column

## Signature

```> dfr rename-col (from) (to)```

## Parameters

 -  `from`: column name to be renamed
 -  `to`: new column name

## Examples

Renames a dataframe column
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr rename-col a a_new
```
