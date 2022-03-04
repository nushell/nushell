---
title: reject
layout: command
version: 0.59.1
---

Remove the given columns from the table. If you want to remove rows, try 'drop'.

## Signature

```> reject ...rest```

## Parameters

 -  `...rest`: the names of columns to remove from the table

## Examples

Lists the files in a directory without showing the modified column
```shell
> ls | reject modified
```

Reject the specified field in a record
```shell
> echo {a: 1, b: 2} | reject a
```
