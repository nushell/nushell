---
title: keep until
layout: command
version: 0.59.0
---

Keep elements of the input until a predicate is true.

## Signature

```> keep until (predicate)```

## Parameters

 -  `predicate`: the predicate that kept element must not match

## Examples

Keep until the element is positive
```shell
> echo [-1 -2 9 1] | keep until $it > 0
```
