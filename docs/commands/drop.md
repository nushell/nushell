---
title: drop
layout: command
version: 0.59.1
---

Remove the last number of rows or columns.

## Signature

```> drop (rows)```

## Parameters

 -  `rows`: starting from the back, the number of rows to remove

## Examples

Remove the last item of a list/table
```shell
> [0,1,2,3] | drop
```

Remove zero item of a list/table
```shell
> [0,1,2,3] | drop 0
```

Remove the last two items of a list/table
```shell
> [0,1,2,3] | drop 2
```
