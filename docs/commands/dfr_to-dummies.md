---
title: dfr to-dummies
layout: command
version: 0.59.0
---

Creates a new dataframe with dummy variables

## Signature

```> dfr to-dummies ```

## Examples

Create new dataframe with dummy variables from a dataframe
```shell
> [[a b]; [1 2] [3 4]] | dfr to-df | dfr to-dummies
```

Create new dataframe with dummy variables from a series
```shell
> [1 2 2 3 3] | dfr to-df | dfr to-dummies
```
