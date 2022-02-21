---
title: seq date
layout: command
version: 0.59.0
---

print sequences of dates

## Signature

```> seq date --separator --output-format --input-format --begin-date --end-date --increment --days --reverse```

## Parameters

 -  `--separator {string}`: separator character (defaults to \n)
 -  `--output-format {string}`: prints dates in this format (defaults to %Y-%m-%d)
 -  `--input-format {string}`: give argument dates in this format (defaults to %Y-%m-%d)
 -  `--begin-date {string}`: beginning date range
 -  `--end-date {string}`: ending date
 -  `--increment {int}`: increment dates by this number
 -  `--days {int}`: number of days to print
 -  `--reverse`: print dates in reverse

## Examples

print the next 10 days in YYYY-MM-DD format with newline separator
```shell
> seq date --days 10
```

print the previous 10 days in YYYY-MM-DD format with newline separator
```shell
> seq date --days 10 -r
```

print the previous 10 days starting today in MM/DD/YYYY format with newline separator
```shell
> seq date --days 10 -o '%m/%d/%Y' -r
```

print the first 10 days in January, 2020
```shell
> seq date -b '2020-01-01' -e '2020-01-10'
```

print every fifth day between January 1st 2020 and January 31st 2020
```shell
> seq date -b '2020-01-01' -e '2020-01-31' -n 5
```

starting on May 5th, 2020, print the next 10 days in your locale's date format, colon separated
```shell
> seq date -o %x -s ':' -d 10 -b '2020-05-01'
```
