---
title: str length
layout: command
version: 0.59.0
---

outputs the lengths of the strings in the pipeline

## Signature

```> str length ...rest```

## Parameters

 -  `...rest`: optionally find length of text by column paths

## Examples

Return the lengths of multiple strings
```shell
> 'hello' | str length
```

Return the lengths of multiple strings
```shell
> ['hi' 'there'] | str length
```
