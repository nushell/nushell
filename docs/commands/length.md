---
title: length
layout: command
version: 0.59.0
---

Count the number of elements in the input.

## Signature

```> length --column```

## Parameters

 -  `--column`: Show the number of columns in a table

## Examples

Count the number of entries in a list
```shell
> echo [1 2 3 4 5] | length
```

Count the number of columns in the calendar table
```shell
> cal | length -c
```
