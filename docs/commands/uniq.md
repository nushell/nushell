---
title: uniq
layout: command
version: 0.59.1
---

Return the unique rows.

## Signature

```> uniq --count --repeated --ignore-case --unique```

## Parameters

 -  `--count`: Count the unique rows
 -  `--repeated`: Count the rows that has more than one value
 -  `--ignore-case`: Ignore differences in case when comparing
 -  `--unique`: Only return unique values

## Examples

Remove duplicate rows of a list/table
```shell
> [2 3 3 4] | uniq
```

Only print duplicate lines, one for each group
```shell
> [1 2 2] | uniq -d
```

Only print unique lines lines
```shell
> [1 2 2] | uniq -u
```

Ignore differences in case when comparing
```shell
> ['hello' 'goodbye' 'Hello'] | uniq -i
```

Remove duplicate rows and show counts of a list/table
```shell
> [1 2 2] | uniq -c
```
