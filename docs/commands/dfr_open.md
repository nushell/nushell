---
title: dfr open
layout: command
version: 0.59.1
---

Opens csv, json or parquet file to create dataframe

## Signature

```> dfr open (file) --delimiter --no-header --infer-schema --skip-rows --columns```

## Parameters

 -  `file`: file path to load values from
 -  `--delimiter {string}`: file delimiter character. CSV file
 -  `--no-header`: Indicates if file doesn't have header. CSV file
 -  `--infer-schema {number}`: Number of rows to infer the schema of the file. CSV file
 -  `--skip-rows {number}`: Number of rows to skip from file. CSV file
 -  `--columns {list<string>}`: Columns to be selected from csv file. CSV and Parquet file

## Examples

Takes a file name and creates a dataframe
```shell
> dfr open test.csv
```
