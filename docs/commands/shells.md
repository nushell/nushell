---
title: shells
layout: command
version: 0.59.0
---

Lists all open shells.

## Signature

```> shells ```

## Examples

Enter a new shell at parent path '..' and show all opened shells
```shell
> enter ..; shells
```

Show currently active shell
```shell
> shells | where active == $true
```
