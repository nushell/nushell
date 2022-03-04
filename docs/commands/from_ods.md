---
title: from ods
layout: command
version: 0.59.1
---

Parse OpenDocument Spreadsheet(.ods) data and create table.

## Signature

```> from ods --sheets```

## Parameters

 -  `--sheets {list<string>}`: Only convert specified sheets

## Examples

Convert binary .ods data to a table
```shell
> open test.txt | from ods
```

Convert binary .ods data to a table, specifying the tables
```shell
> open test.txt | from ods -s [Spreadsheet1]
```
