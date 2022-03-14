---
title: where
layout: command
version: 0.59.1
---

Filter values based on a condition.

## Signature

```> where (cond)```

## Parameters

 -  `cond`: condition

## Examples

List all files in the current directory with sizes greater than 2kb
```shell
> ls | where size > 2kb
```

List only the files in the current directory
```shell
> ls | where type == file
```

List all files with names that contain "Car"
```shell
> ls | where name =~ "Car"
```

List all files that were modified in the last two weeks
```shell
> ls | where modified >= (date now) - 2wk
```
