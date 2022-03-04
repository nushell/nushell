---
title: size
layout: command
version: 0.59.1
---

Gather word count statistics on the text.

## Signature

```> size ```

## Examples

Count the number of words in a string
```shell
> "There are seven words in this sentence" | size
```

Counts Unicode characters correctly in a string
```shell
> "Amélie Amelie" | size
```
