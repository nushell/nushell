---
title: sort-by
layout: command
version: 0.59.1
---

Sort by the given columns, in increasing order.

## Signature

```> sort-by ...columns --reverse --insensitive```

## Parameters

 -  `...columns`: the column(s) to sort by
 -  `--reverse`: Sort in reverse order
 -  `--insensitive`: Sort string-based columns case-insensitively

## Examples

sort the list by increasing value
```shell
> [2 0 1] | sort-by
```

sort the list by decreasing value
```shell
> [2 0 1] | sort-by -r
```

sort a list of strings
```shell
> [betty amy sarah] | sort-by
```

sort a list of strings in reverse
```shell
> [betty amy sarah] | sort-by -r
```

Sort strings (case-insensitive)
```shell
> echo [airplane Truck Car] | sort-by -i
```

Sort strings (reversed case-insensitive)
```shell
> echo [airplane Truck Car] | sort-by -i -r
```
