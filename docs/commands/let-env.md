---
title: let-env
layout: command
version: 0.59.0
---

Create an environment variable and give it a value.

## Signature

```> let-env (var_name) (initial_value)```

## Parameters

 -  `var_name`: variable name
 -  `initial_value`: equals sign followed by value

## Examples

Create an environment variable and display it
```shell
> let-env MY_ENV_VAR = 1; $env.MY_ENV_VAR
```
