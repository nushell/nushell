---
title: decode
layout: command
version: 0.59.0
---

Decode bytes as a string.

## Signature

```> decode (encoding)```

## Parameters

 -  `encoding`: the text encoding to use

## Examples

Decode the output of an external command
```shell
> cat myfile.q | decode utf-8
```
