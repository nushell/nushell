---
title: dfr concatenate
layout: command
version: 0.59.1
---

Concatenates strings with other array

## Signature

```> dfr concatenate (other)```

## Parameters

 -  `other`: Other array with string to be concatenated

## Examples

Concatenate string
```shell
> let other = ([za xs cd] | dfr to-df);
    [abc abc abc] | dfr to-df | dfr concatenate $other
```
