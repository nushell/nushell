---
title: dfr is-duplicated
layout: command
version: 0.59.0
---

Creates mask indicating duplicated values

## Signature

```> dfr is-duplicated ```

## Examples

Create mask indicating duplicated values
```shell
> [5 6 6 6 8 8 8] | dfr to-df | dfr is-duplicated
```
