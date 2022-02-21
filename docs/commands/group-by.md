---
title: group-by
layout: command
version: 0.59.0
---

Create a new table grouped.

## Signature

```> group-by (grouper)```

## Parameters

 -  `grouper`: the grouper value to use

## Examples

group items by column named "type"
```shell
> ls | group-by type
```

you can also group by raw values by leaving out the argument
```shell
> echo ['1' '3' '1' '3' '2' '1' '1'] | group-by
```
