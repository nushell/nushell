---
title: dfr is-in
layout: command
version: 0.59.0
---

Checks if elements from a series are contained in right series

## Signature

```> dfr is-in (other)```

## Parameters

 -  `other`: right series

## Examples

Checks if elements from a series are contained in right series
```shell
> let other = ([1 3 6] | dfr to-df);
    [5 6 6 6 8 8 8] | dfr to-df | dfr is-in $other
```
