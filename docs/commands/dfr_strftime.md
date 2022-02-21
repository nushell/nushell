---
title: dfr strftime
layout: command
version: 0.59.0
---

Formats date based on string rule

## Signature

```> dfr strftime (fmt)```

## Parameters

 -  `fmt`: Format rule

## Examples

Formats date
```shell
> let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr to-df);
    $df | dfr strftime "%Y/%m/%d"
```
