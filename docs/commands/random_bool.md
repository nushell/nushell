---
title: random bool
layout: command
version: 0.59.0
---

Generate a random boolean value

## Signature

```> random bool --bias```

## Parameters

 -  `--bias {number}`: Adjusts the probability of a "true" outcome

## Examples

Generate a random boolean value
```shell
> random bool
```

Generate a random boolean value with a 75% chance of "true"
```shell
> random bool --bias 0.75
```
