---
title: path split
layout: command
version: 0.59.0
---

Split a path into parts by a separator.

## Signature

```> path split --columns```

## Parameters

 -  `--columns {table}`: Optionally operate by column path

## Examples

Split a path into parts
```shell
> '/home/viking/spam.txt' | path split
```

Split all paths under the 'name' column
```shell
> ls ('.' | path expand) | path split -c [ name ]
```
