---
title: each group
layout: command
version: 0.59.0
---

Runs a block on groups of `group_size` rows of a table at a time.

## Signature

```> each group (group_size) (block)```

## Parameters

 -  `group_size`: the size of each group
 -  `block`: the block to run on each group

## Examples

Echo the sum of each pair
```shell
> echo [1 2 3 4] | each group 2 { |it| $it.0 + $it.1 }
```
