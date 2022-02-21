---
title: from json
layout: command
version: 0.59.0
---

Convert from json to structured data

## Signature

```> from json --objects```

## Parameters

 -  `--objects`: treat each line as a separate value

## Examples

Converts json formatted string to table
```shell
> '{ "a": 1 }' | from json
```

Converts json formatted string to table
```shell
> '{ "a": 1, "b": [1, 2] }' | from json
```
