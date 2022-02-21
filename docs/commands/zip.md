---
title: zip
layout: command
version: 0.59.0
---

Combine a stream with the input

## Signature

```> zip (other)```

## Parameters

 -  `other`: the other input

## Examples

Zip multiple streams and get one of the results
```shell
> 1..3 | zip 4..6
```
