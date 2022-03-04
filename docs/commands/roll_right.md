---
title: roll right
layout: command
version: 0.59.1
---

Roll table columns right

## Signature

```> roll right --by --cells-only```

## Parameters

 -  `--by {int}`: Number of columns to roll
 -  `--cells-only`: rotates columns leaving headers fixed

## Examples

Rolls columns to the right
```shell
> [[a b c]; [1 2 3] [4 5 6]] | roll right
```

Rolls columns to the right with fixed headers
```shell
> [[a b c]; [1 2 3] [4 5 6]] | roll right --cells-only
```
