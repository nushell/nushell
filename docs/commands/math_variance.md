---
title: math variance
layout: command
version: 0.59.1
---

Finds the variance of a list of numbers or tables

## Signature

```> math variance --sample```

## Parameters

 -  `--sample`: calculate sample variance

## Examples

Get the variance of a list of numbers
```shell
> echo [1 2 3 4 5] | math variance
```

Get the sample variance of a list of numbers
```shell
> [1 2 3 4 5] | math variance -s
```
