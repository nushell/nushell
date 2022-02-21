---
title: exec
layout: command
version: 0.59.0
---

Execute a command, replacing the current process.

## Signature

```> exec (command) ...rest```

## Parameters

 -  `command`: the command to execute
 -  `...rest`: any additional arguments for the command

## Examples

Execute external 'ps aux' tool
```shell
> exec ps aux
```

Execute 'nautilus'
```shell
> exec nautilus
```
