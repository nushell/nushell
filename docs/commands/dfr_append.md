---
title: dfr append
layout: command
version: 0.59.1
---

Appends a new dataframe

## Signature

```> dfr append (other) --col```

## Parameters

 -  `other`: dataframe to be appended
 -  `--col`: appends in col orientation

## Examples

Appends a dataframe as new columns
```shell
> let a = ([[a b]; [1 2] [3 4]] | dfr to-df);
    $a | dfr append $a
```

Appends a dataframe merging at the end of columns
```shell
> let a = ([[a b]; [1 2] [3 4]] | dfr to-df);
    $a | dfr append $a --col
```
