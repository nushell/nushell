---
title: from yaml
layout: command
version: 0.59.0
---

Parse text as .yaml/.yml and create table.

## Signature

```> from yaml ```

## Examples

Converts yaml formatted string to table
```shell
> 'a: 1' | from yaml
```

Converts yaml formatted string to table
```shell
> '[ a: 1, b: [1, 2] ]' | from yaml
```
