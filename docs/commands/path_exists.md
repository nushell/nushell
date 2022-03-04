---
title: path exists
layout: command
version: 0.59.1
---

Check whether a path exists

## Signature

```> path exists --columns```

## Parameters

 -  `--columns {table}`: Optionally operate by column path

## Examples

Check if a file exists
```shell
> '/home/joe/todo.txt' | path exists
```

Check if a file exists in a column
```shell
> ls | path exists -c [ name ]
```
