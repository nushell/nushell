---
title: from toml
layout: command
version: 0.59.0
---

Parse text as .toml and create table.

## Signature

```> from toml ```

## Examples

Converts toml formatted string to table
```shell
> 'a = 1' | from toml
```

Converts toml formatted string to table
```shell
> 'a = 1
b = [1, 2]' | from toml
```
