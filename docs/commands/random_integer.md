---
title: random integer
layout: command
version: 0.59.0
---

Generate a random integer [min..max]

## Signature

```> random integer (range)```

## Parameters

 -  `range`: Range of values

## Examples

Generate an unconstrained random integer
```shell
> random integer
```

Generate a random integer less than or equal to 500
```shell
> random integer ..500
```

Generate a random integer greater than or equal to 100000
```shell
> random integer 100000..
```

Generate a random integer between 1 and 10
```shell
> random integer 1..10
```
