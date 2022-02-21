---
title: dfr count-null
layout: command
version: 0.59.0
---

Counts null values

## Signature

```> dfr count-null ```

## Examples

Counts null values
```shell
> let s = ([1 1 0 0 3 3 4] | dfr to-df);
    ($s / $s) | dfr count-null
```
