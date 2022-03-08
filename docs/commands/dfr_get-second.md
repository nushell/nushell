---
title: dfr get-second
layout: command
version: 0.59.1
---

Gets second from date

## Signature

```> dfr get-second ```

## Examples

Returns second from a date
```shell
> let dt = ('2020-08-04T16:39:18+00:00' | into datetime -z 'UTC');
    let df = ([$dt $dt] | dfr to-df);
    $df | dfr get-second
```
