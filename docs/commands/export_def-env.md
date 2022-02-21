---
title: export def-env
layout: command
version: 0.59.0
---

Define a custom command that participates in the environment and export it from a module

## Signature

```> export def-env (name) (params) (block)```

## Parameters

 -  `name`: definition name
 -  `params`: parameters
 -  `block`: body of the definition

## Examples

Define a custom command that participates in the environment in a module and call it
```shell
> module foo { export def-env bar [] { let-env FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR
```
