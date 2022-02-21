---
title: dfr column
layout: command
version: 0.59.0
---

Returns the selected column

## Signature

```> dfr column (column)```

## Parameters

 -  `column`: column name

## Examples

Returns the selected column as series
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr column a
```
