---
title: lines
layout: command
version: 0.59.1
---

Converts input to lines

## Signature

```> lines --skip-empty```

## Parameters

 -  `--skip-empty`: skip empty lines

## Examples

Split multi-line string into lines
```shell
> echo $'two(char nl)lines' | lines
```
