---
title: dfr to-nu
layout: command
version: 0.59.0
---

Converts a section of the dataframe to Nushell Table

## Signature

```> dfr to-nu --n-rows --tail```

## Parameters

 -  `--n-rows {number}`: number of rows to be shown
 -  `--tail`: shows tail rows

## Examples

Shows head rows from dataframe
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr to-nu
```

Shows tail rows from dataframe
```shell
> [[a b]; [1 2] [3 4] [5 6]] | dfr to-df | dfr to-nu -t -n 1
```
