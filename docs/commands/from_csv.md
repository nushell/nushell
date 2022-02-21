---
title: from csv
layout: command
version: 0.59.0
---

Parse text as .csv and create table.

## Signature

```> from csv --separator --noheaders```

## Parameters

 -  `--separator {string}`: a character to separate columns, defaults to ','
 -  `--noheaders`: don't treat the first row as column names

## Examples

Convert comma-separated data to a table
```shell
> open data.txt | from csv
```

Convert comma-separated data to a table, ignoring headers
```shell
> open data.txt | from csv --noheaders
```

Convert comma-separated data to a table, ignoring headers
```shell
> open data.txt | from csv -n
```

Convert semicolon-separated data to a table
```shell
> open data.txt | from csv --separator ';'
```
