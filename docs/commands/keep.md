---
title: keep
layout: command
version: 0.59.0
---

Keep the first n elements of the input.

## Signature

```> keep (n)```

## Parameters

 -  `n`: the number of elements to keep

## Examples

Keep two elements
```shell
> echo [[editions]; [2015] [2018] [2021]] | keep 2
```

Keep the first value
```shell
> echo [2 4 6 8] | keep
```
