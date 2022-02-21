---
title: to md
layout: command
version: 0.59.0
---

Convert table into simple Markdown

## Signature

```> to md --pretty --per-element```

## Parameters

 -  `--pretty`: Formats the Markdown table to vertically align items
 -  `--per-element`: treat each row as markdown syntax element

## Examples

Outputs an MD string representing the contents of this table
```shell
> [[foo bar]; [1 2]] | to md
```

Optionally, output a formatted markdown string
```shell
> [[foo bar]; [1 2]] | to md --pretty
```

Treat each row as a markdown element
```shell
> [{"H1": "Welcome to Nushell" } [[foo bar]; [1 2]]] | to md --per-element --pretty
```
