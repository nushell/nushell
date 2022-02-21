---
title: ansi strip
layout: command
version: 0.59.0
---

strip ansi escape sequences from string

## Signature

```> ansi strip ...column path```

## Parameters

 -  `...column path`: optionally, remove ansi sequences by column paths

## Examples

strip ansi escape sequences from string
```shell
> echo [ (ansi green) (ansi cursor_on) "hello" ] | str collect | ansi strip
```
