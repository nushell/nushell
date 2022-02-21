---
title: benchmark
layout: command
version: 0.59.0
---

Time the running time of a block

## Signature

```> benchmark (block)```

## Parameters

 -  `block`: the block to run

## Examples

Benchmarks a command within a block
```shell
> benchmark { sleep 500ms }
```
