---
title: dfr is-not-null
layout: command
version: 0.59.1
---

Creates mask where value is not null

## Signature

```> dfr is-not-null ```

## Examples

Create mask where values are not null
```shell
> let s = ([5 6 0 8] | dfr to-df);
    let res = ($s / $s);
    $res | dfr is-not-null
```
