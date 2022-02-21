---
title: par-each group
layout: command
version: 0.59.0
---

Runs a block on groups of `group_size` rows of a table at a time.

## Signature

```> par-each group (group_size) (block)```

## Parameters

 -  `group_size`: the size of each group
 -  `block`: the block to run on each group

## Examples

Multiplies elements in list
```shell
> echo [1 2 3 4] | par-each group 2 { $it.0 + $it.1 }
```
