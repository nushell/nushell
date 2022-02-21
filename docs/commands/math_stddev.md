---
title: math stddev
layout: command
version: 0.59.0
---

Finds the stddev of a list of numbers or tables

## Signature

```> math stddev --sample```

## Parameters

 -  `--sample`: calculate sample standard deviation

## Examples

Get the stddev of a list of numbers
```shell
> [1 2 3 4 5] | math stddev
```

Get the sample stddev of a list of numbers
```shell
> [1 2 3 4 5] | math stddev -s
```
