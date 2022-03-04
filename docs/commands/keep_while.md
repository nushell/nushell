---
title: keep while
layout: command
version: 0.59.1
---

Keep elements of the input while a predicate is true.

## Signature

```> keep while (predicate)```

## Parameters

 -  `predicate`: the predicate that kept element must not match

## Examples

Keep while the element is negative
```shell
> echo [-1 -2 9 1] | keep while $it < 0
```
