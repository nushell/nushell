---
title: table
layout: command
version: 0.59.0
---

Render the table.

## Signature

```> table --start-number```

## Parameters

 -  `--start-number {int}`: row number to start viewing from

## Examples

List the files in current directory with index number start from 1.
```shell
> ls | table -n 1
```

Render data in table view
```shell
> echo [[a b]; [1 2] [3 4]] | table
```
