---
title: p
layout: command
version: 0.59.1
---

Switch to the previous shell.

## Signature

```> p ```

## Examples

Make two directories and enter new shells for them, use `p` to jump to the previous shell
```shell
> mkdir foo bar; enter foo; enter ../bar; p
```

Run `p` several times and note the changes of current directory
```shell
> p
```
