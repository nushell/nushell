---
title: default
layout: command
version: 0.59.1
---

Sets a default row's column if missing.

## Signature

```> default (default value) (column name)```

## Parameters

 -  `default value`: the value to use as a default
 -  `column name`: the name of the column

## Examples

Give a default 'target' column to all file entries
```shell
> ls -la | default 'nothing' target
```

Default the `$nothing` value in a list
```shell
> [1, 2, $nothing, 4] | default 3
```
