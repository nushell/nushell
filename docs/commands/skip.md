---
title: skip
layout: command
version: 0.59.0
---

Skip the first n elements of the input.

## Signature

```> skip (n)```

## Parameters

 -  `n`: the number of elements to skip

## Examples

Skip two elements
```shell
> echo [[editions]; [2015] [2018] [2021]] | skip 2
```

Skip the first value
```shell
> echo [2 4 6 8] | skip
```
