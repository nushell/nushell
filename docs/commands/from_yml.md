---
title: from yml
layout: command
version: 0.59.1
---

Parse text as .yaml/.yml and create table.

## Signature

```> from yml ```

## Examples

Converts yaml formatted string to table
```shell
> 'a: 1' | from yaml
```

Converts yaml formatted string to table
```shell
> '[ a: 1, b: [1, 2] ]' | from yaml
```
