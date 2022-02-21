---
title: str substring
layout: command
version: 0.59.0
---

substrings text

## Signature

```> str substring (range) ...rest```

## Parameters

 -  `range`: the indexes to substring [start end]
 -  `...rest`: optionally substring text by column paths

## Examples

Get a substring from the text
```shell
>  'good nushell' | str substring [5 12]
```

Alternatively, you can use the form
```shell
>  'good nushell' | str substring '5,12'
```

Drop the last `n` characters from the string
```shell
>  'good nushell' | str substring ',-5'
```

Get the remaining characters from a starting index
```shell
>  'good nushell' | str substring '5,'
```

Get the characters from the beginning until ending index
```shell
>  'good nushell' | str substring ',7'
```
