---
title: from tsv
layout: command
version: 0.59.1
---

Parse text as .tsv and create table.

## Signature

```> from tsv --noheaders```

## Parameters

 -  `--noheaders`: don't treat the first row as column names

## Examples

Create a tsv file with header columns and open it
```shell
> echo $'c1(char tab)c2(char tab)c3(char nl)1(char tab)2(char tab)3' | save tsv-data | open tsv-data | from tsv
```

Create a tsv file without header columns and open it
```shell
> echo $'a1(char tab)b1(char tab)c1(char nl)a2(char tab)b2(char tab)c2' | save tsv-data | open tsv-data | from tsv -n
```
