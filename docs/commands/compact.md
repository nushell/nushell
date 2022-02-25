---
title: compact
layout: command
version: 0.59.0
---

Creates a table with non-empty rows.

## Signature

```> compact ...columns```

## Parameters

 -  `...columns`: the columns to compact from the table

## Examples

Filter out all records where 'Hello' is null (returns nothing)
```shell
> echo [["Hello" "World"]; [$nothing 3]]| compact Hello
```

Filter out all records where 'World' is null (Returns the table)
```shell
> echo [["Hello" "World"]; [$nothing 3]]| compact World
```

Filter out all instances of nothing from a list (Returns [1,2])
```shell
> echo [1, $nothing, 2] | compact
```
