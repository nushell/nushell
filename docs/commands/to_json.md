---
title: to json
layout: command
version: 0.59.0
---

Converts table data into JSON text.

## Signature

```> to json --raw```

## Parameters

 -  `--raw`: remove all of the whitespace

## Examples

Outputs an unformatted JSON string representing the contents of this table
```shell
> [1 2 3] | to json
```

