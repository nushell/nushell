---
title: parse
layout: command
version: 0.59.0
---

Parse columns from string data using a simple pattern.

## Signature

```> parse (pattern) --regex```

## Parameters

 -  `pattern`: the pattern to match. Eg) "{foo}: {bar}"
 -  `--regex`: use full regex syntax for patterns

## Examples

Parse a string into two named columns
```shell
> echo "hi there" | parse "{foo} {bar}"
```

Parse a string using regex pattern
```shell
> echo "hi there" | parse -r "(?P<foo>\w+) (?P<bar>\w+)"
```
