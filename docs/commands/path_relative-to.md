---
title: path relative-to
layout: command
version: 0.59.1
---

Get a path as relative to another path.

## Signature

```> path relative-to (path) --columns```

## Parameters

 -  `path`: Parent shared with the input path
 -  `--columns {table}`: Optionally operate by column path

## Examples

Find a relative path from two absolute paths
```shell
> '/home/viking' | path relative-to '/home'
```

Find a relative path from two absolute paths in a column
```shell
> ls ~ | path relative-to ~ -c [ name ]
```

Find a relative path from two relative paths
```shell
> 'eggs/bacon/sausage/spam' | path relative-to 'eggs/bacon/sausage'
```
