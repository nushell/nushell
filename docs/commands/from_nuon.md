---
title: from nuon
layout: command
version: 0.59.1
---

Convert from nuon to structured data

## Signature

```> from nuon ```

## Examples

Converts nuon formatted string to table
```shell
> '{ a:1 }' | from nuon
```

Converts nuon formatted string to table
```shell
> '{ a:1, b: [1, 2] }' | from nuon
```
