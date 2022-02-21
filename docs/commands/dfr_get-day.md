---
title: dfr get-day
layout: command
version: 0.59.0
---

Gets day from date

## Signature

```> dfr get-day ```

## Examples

Returns day from a date
```shell
> let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr to-df);
    $df | dfr get-day
```
