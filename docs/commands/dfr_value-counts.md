---
title: dfr value-counts
layout: command
version: 0.59.0
---

Returns a dataframe with the counts for unique values in series

## Signature

```> dfr value-counts ```

## Examples

Calculates value counts
```shell
> [5 5 5 5 6 6] | dfr to-df | dfr value-counts
```
