---
title: path type
layout: command
version: 0.59.1
---

Get the type of the object a path refers to (e.g., file, dir, symlink)

## Signature

```> path type --columns```

## Parameters

 -  `--columns {table}`: Optionally operate by column path

## Examples

Show type of a filepath
```shell
> '.' | path type
```

Show type of a filepath in a column
```shell
> ls | path type -c [ name ]
```
