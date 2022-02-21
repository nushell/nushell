---
title: dfr take
layout: command
version: 0.59.0
---

Creates new dataframe using the given indices

## Signature

```> dfr take (indices)```

## Parameters

 -  `indices`: list of indices used to take data

## Examples

Takes selected rows from dataframe
```shell
> let df = ([[a b]; [4 1] [5 2] [4 3]] | dfr to-df);
    let indices = ([0 2] | dfr to-df);
    $df | dfr take $indices
```

Takes selected rows from series
```shell
> let series = ([4 1 5 2 4 3] | dfr to-df);
    let indices = ([0 2] | dfr to-df);
    $series | dfr take $indices
```
