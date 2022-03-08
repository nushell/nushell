---
title: dfr replace-all
layout: command
version: 0.59.1
---

Replace all (sub)strings by a regex pattern

## Signature

```> dfr replace-all --pattern --replace```

## Parameters

 -  `--pattern {string}`: Regex pattern to be matched
 -  `--replace {string}`: replacing string

## Examples

Replaces string
```shell
> [abac abac abac] | dfr to-df | dfr replace-all -p a -r A
```
