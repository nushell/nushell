---
title: dfr contains
layout: command
version: 0.59.0
---

Checks if a pattern is contained in a string

## Signature

```> dfr contains (pattern)```

## Parameters

 -  `pattern`: Regex pattern to be searched

## Examples

Returns boolean indicating if pattern was found
```shell
> [abc acb acb] | dfr to-df | dfr contains ab
```
