---
title: split row
layout: command
version: 0.59.0
---

splits contents over multiple rows via the separator.

## Signature

```> split row (separator)```

## Parameters

 -  `separator`: the character that denotes what separates rows

## Examples

Split a string into rows of char
```shell
> echo 'abc' | split row ''
```

Split a string into rows by the specified separator
```shell
> echo 'a--b--c' | split row '--'
```
