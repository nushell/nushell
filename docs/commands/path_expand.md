---
title: path expand
layout: command
version: 0.59.0
---

Try to expand a path to its absolute form

## Signature

```> path expand --strict --columns```

## Parameters

 -  `--strict`: Throw an error if the path could not be expanded
 -  `--columns {table}`: Optionally operate by column path

## Examples

Expand an absolute path
```shell
> '/home/joe/foo/../bar' | path expand
```

Expand a path in a column
```shell
> ls | path expand -c [ name ]
```

Expand a relative path
```shell
> 'foo/../bar' | path expand
```
