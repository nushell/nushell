---
title: default
layout: command
version: 0.59.1
---

Sets a default row's column if missing.

## Signature

```> default (column name) (column value)```

## Parameters

 -  `column name`: the name of the column
 -  `column value`: the value of the column to default

## Examples

Give a default 'target' to all file entries
```shell
> ls -la | default target 'nothing'
```
