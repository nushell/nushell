---
title: each window
layout: command
version: 0.59.1
---

Runs a block on window groups of `window_size` that slide by n rows.

## Signature

```> each window (window_size) (block) --stride```

## Parameters

 -  `window_size`: the size of each window
 -  `block`: the block to run on each window
 -  `--stride {int}`: the number of rows to slide over between windows

## Examples

A sliding window of two elements
```shell
> echo [1 2 3 4] | each window 2 { |it| $it.0 + $it.1 }
```

A sliding window of two elements, with a stride of 3
```shell
> [1, 2, 3, 4, 5, 6, 7, 8] | each window 2 --stride 3 { |x| $x.0 + $x.1 }
```
