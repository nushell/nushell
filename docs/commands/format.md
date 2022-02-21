---
title: format
layout: command
version: 0.59.0
---

Format columns into a string using a simple pattern.

## Signature

```> format (pattern)```

## Parameters

 -  `pattern`: the pattern to output. e.g.) "{foo}: {bar}"

## Examples

Print filenames with their sizes
```shell
> ls | format '{name}: {size}'
```

Print elements from some columns of a table
```shell
> echo [[col1, col2]; [v1, v2] [v3, v4]] | format '{col2}'
```
