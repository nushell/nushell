---
title: dfr get-weekday
layout: command
version: 0.59.1
---

Gets weekday from date

## Signature

```> dfr get-weekday ```

## Examples

Returns weekday from a date
```shell
> let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr to-df);
    $df | dfr get-weekday
```
