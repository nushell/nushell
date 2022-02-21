---
title: skip until
layout: command
version: 0.59.0
---

Skip elements of the input until a predicate is true.

## Signature

```> skip until (predicate)```

## Parameters

 -  `predicate`: the predicate that skipped element must not match

## Examples

Skip until the element is positive
```shell
> echo [-2 0 2 -1] | skip until $it > 0
```
