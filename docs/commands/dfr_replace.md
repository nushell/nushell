---
title: dfr replace
layout: command
version: 0.59.1
---

Replace the leftmost (sub)string by a regex pattern

## Signature

```> dfr replace --pattern --replace```

## Parameters

 -  `--pattern {string}`: Regex pattern to be matched
 -  `--replace {string}`: replacing string

## Examples

Replaces string
```shell
> [abc abc abc] | dfr to-df | dfr replace -p ab -r AB
```
