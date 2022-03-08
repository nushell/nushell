---
title: run-external
layout: command
version: 0.59.1
---

Runs external command

## Signature

```> run-external ...rest --redirect-stdout --redirect-stderr```

## Parameters

 -  `...rest`: external command to run
 -  `--redirect-stdout`: redirect-stdout
 -  `--redirect-stderr`: redirect-stderr

## Examples

Run an external command
```shell
> run-external "echo" "-n" "hello"
```
