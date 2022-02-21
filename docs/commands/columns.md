---
title: columns
layout: command
version: 0.59.0
---

Show the columns in the input.

## Signature

```> columns ```

## Examples

Get the columns from the table
```shell
> [[name,age,grade]; [bill,20,a]] | columns
```

Get the first column from the table
```shell
> [[name,age,grade]; [bill,20,a]] | columns | first
```

Get the second column from the table
```shell
> [[name,age,grade]; [bill,20,a]] | columns | select 1
```
