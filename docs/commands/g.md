---
title: g
layout: command
version: 0.59.0
---

Switch to a given shell.

## Signature

```> g (shell_number)```

## Parameters

 -  `shell_number`: shell number to change to

## Examples

Make two directories and enter new shells for them, use `g` to jump to the specific shell
```shell
> mkdir foo bar; enter foo; enter ../bar; g 1
```

Use `shells` to show all the opened shells and run `g 2` to jump to the third one
```shell
> shells; g 2
```
