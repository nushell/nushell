---
title: to json
layout: command
version: 0.59.0
---

Converts table data into JSON text.

## Signature

```> to json --raw --indent```

## Parameters

 -  `--raw`: remove all of the whitespace
 -  `--indent {number}`: specify indentation width

## Examples

Outputs a JSON string, with default indentation, representing the contents of this table
```shell
> [a b c] | to json
```

Outputs a JSON string, with 4-space indentation, representing the contents of this table
```shell
> [Joe Bob Sam] | to json -i 4
```

Outputs an unformatted JSON string representing the contents of this table
```shell
> [1 2 3] | to json -r
```
