---
title: for
layout: command
version: 0.59.0
---

Loop over a range

## Signature

```> for (var_name) (range) (block) --numbered```

## Parameters

 -  `var_name`: name of the looping variable
 -  `range`: range of the loop
 -  `block`: the block to run
 -  `--numbered`: returned a numbered item ($it.index and $it.item)

## Examples

Echo the square of each integer
```shell
> for x in [1 2 3] { $x * $x }
```

Work with elements of a range
```shell
> for $x in 1..3 { $x }
```

Number each item and echo a message
```shell
> for $it in ['bob' 'fred'] --numbered { $"($it.index) is ($it.item)" }
```
