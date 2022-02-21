---
title: collect
layout: command
version: 0.59.0
---

Collect the stream and pass it to a block.

## Signature

```> collect (block)```

## Parameters

 -  `block`: the block to run once the stream is collected

## Examples

Use the second value in the stream
```shell
> echo 1 2 3 | collect { |x| echo $x.1 }
```
