---
title: each
layout: command
version: 0.59.0
---

Run a block on each element of input

## Signature

```> each (block) --keep-empty --numbered```

## Parameters

 -  `block`: the block to run
 -  `--keep-empty`: keep empty result cells
 -  `--numbered`: iterate with an index

## Examples

Multiplies elements in list
```shell
> [1 2 3] | each { |it| 2 * $it }
```

Iterate over each element, keeping only values that succeed
```shell
> [1 2 3] | each { |it| if $it == 2 { echo "found 2!"} }
```

Iterate over each element, keeping all results
```shell
> [1 2 3] | each --keep-empty { |it| if $it == 2 { echo "found 2!"} }
```
