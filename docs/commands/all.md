---
title: all?
layout: command
version: 0.59.0
---

Test if every element of the input matches a predicate.

## Signature

```> all? (predicate)```

## Parameters

 -  `predicate`: the predicate that must match

## Examples

Find if services are running
```shell
> echo [[status]; [UP] [UP]] | all? status == UP
```

Check that all values are even
```shell
> echo [2 4 6 8] | all? ($it mod 2) == 0
```
