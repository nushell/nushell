---
title: from xlsx
layout: command
version: 0.59.1
---

Parse binary Excel(.xlsx) data and create table.

## Signature

```> from xlsx --sheets```

## Parameters

 -  `--sheets {list<string>}`: Only convert specified sheets

## Examples

Convert binary .xlsx data to a table
```shell
> open test.txt | from xlsx
```

Convert binary .xlsx data to a table, specifying the tables
```shell
> open test.txt | from xlsx -s [Spreadsheet1]
```
