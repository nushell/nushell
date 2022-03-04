---
title: exit
layout: command
version: 0.59.1
---

Runs a script file in the current context.

## Signature

```> exit (exit_code) --now```

## Parameters

 -  `exit_code`: Exit code to return immediately with
 -  `--now`: Exit out of the shell immediately

## Examples

Exit the current shell
```shell
> exit
```

Exit all shells (exiting Nu)
```shell
> exit --now
```
