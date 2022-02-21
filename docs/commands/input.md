---
title: input
layout: command
version: 0.59.0
---

Get input from the user.

## Signature

```> input (prompt) --bytes-until```

## Parameters

 -  `prompt`: prompt to show the user
 -  `--bytes-until {string}`: read bytes (not text) until a stop byte

## Examples

Get input from the user, and assign to a variable
```shell
> let user-input = (input)
```
