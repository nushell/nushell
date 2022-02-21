---
title: drop column
layout: command
version: 0.59.0
---

Remove the last number of columns. If you want to remove columns by name, try 'reject'.

## Signature

```> drop column (columns)```

## Parameters

 -  `columns`: starting from the end, the number of columns to remove

## Examples

Remove the last column of a table
```shell
> echo [[lib, extension]; [nu-lib, rs] [nu-core, rb]] | drop column
```
