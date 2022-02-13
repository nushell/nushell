---
title: each
layout: command
version: 0.59.0
---

Run a block on each element of input

## Signature

```> each (block) --numbered```

## Parameters

 -  `block`: the block to run
 -  `--numbered`: iterate with an index

## Examples

Multiplies elements in list
```shell
> [1 2 3] | each { 2 * $it }
```

