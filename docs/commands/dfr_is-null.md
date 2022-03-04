---
title: dfr is-null
layout: command
version: 0.59.1
---

Creates mask where value is null

## Signature

```> dfr is-null ```

## Examples

Create mask where values are null
```shell
> let s = ([5 6 0 8] | dfr to-df);
    let res = ($s / $s);
    $res | dfr is-null
```
