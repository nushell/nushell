---
title: grid
layout: command
version: 0.59.1
---

Renders the output to a textual terminal grid.

## Signature

```> grid --width --color --separator```

## Parameters

 -  `--width {int}`: number of terminal columns wide (not output columns)
 -  `--color`: draw output with color
 -  `--separator {string}`: character to separate grid with

## Examples

Render a simple list to a grid
```shell
> [1 2 3 a b c] | grid
```

The above example is the same as:
```shell
> [1 2 3 a b c] | wrap name | grid
```

Render a record to a grid
```shell
> {name: 'foo', b: 1, c: 2} | grid
```

Render a list of records to a grid
```shell
> [{name: 'A', v: 1} {name: 'B', v: 2} {name: 'C', v: 3}] | grid
```

Render a table with 'name' column in it to a grid
```shell
> [[name patch]; [0.1.0 false] [0.1.1 true] [0.2.0 false]] | grid
```
