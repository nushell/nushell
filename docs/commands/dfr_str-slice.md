---
title: dfr str-slice
layout: command
version: 0.59.0
---

Slices the string from the start position until the selected length

## Signature

```> dfr str-slice (start) --length```

## Parameters

 -  `start`: start of slice
 -  `--length {int}`: optional length

## Examples

Creates slices from the strings
```shell
> [abcded abc321 abc123] | dfr to-df | dfr str-slice 1 -l 2
```
