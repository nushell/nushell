---
title: par-each
layout: command
version: 0.59.0
---

Run a block on each element of input in parallel

## Signature

```> par-each (block) --numbered```

## Parameters

 -  `block`: the block to run
 -  `--numbered`: iterate with an index

## Examples

Multiplies elements in list
```shell
> [1 2 3] | par-each { |it| 2 * $it }
```

Iterate over each element, print the matching value and it's index
```shell
> [1 2 3] | par-each -n { |it| if $it.item == 2 { echo $"found 2 at ($it.index)!"} }
```
