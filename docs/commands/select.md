---
title: select
layout: command
version: 0.59.1
---

Down-select table to only these columns.

## Signature

```> select ...rest```

## Parameters

 -  `...rest`: the columns to select from the table

## Examples

Select just the name column
```shell
> ls | select name
```

Select the name and size columns
```shell
> ls | select name size
```
