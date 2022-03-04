---
title: dfr rolling
layout: command
version: 0.59.1
---

Rolling calculation for a series

## Signature

```> dfr rolling (type) (window)```

## Parameters

 -  `type`: rolling operation
 -  `window`: Window size for rolling

## Examples

Rolling sum for a series
```shell
> [1 2 3 4 5] | dfr to-df | dfr rolling sum 2 | dfr drop-nulls
```

Rolling max for a series
```shell
> [1 2 3 4 5] | dfr to-df | dfr rolling max 2 | dfr drop-nulls
```
