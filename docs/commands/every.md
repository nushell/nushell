---
title: every
layout: command
version: 0.59.0
---

Show (or skip) every n-th row, starting from the first one.

## Signature

```> every (stride) --skip```

## Parameters

 -  `stride`: how many rows to skip between (and including) each row returned
 -  `--skip`: skip the rows that would be returned, instead of selecting them

## Examples

Get every second row
```shell
> [1 2 3 4 5] | every 2
```

Skip every second row
```shell
> [1 2 3 4 5] | every 2 --skip
```
