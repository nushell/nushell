---
title: dfr shift
layout: command
version: 0.59.1
---

Shifts the values by a given period

## Signature

```> dfr shift (period)```

## Parameters

 -  `period`: shift period

## Examples

Shifts the values by a given period
```shell
> [1 2 2 3 3] | dfr to-df | dfr shift 2 | dfr drop-nulls
```
