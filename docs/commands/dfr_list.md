---
title: dfr list
layout: command
version: 0.59.1
---

Lists stored dataframes

## Signature

```> dfr list ```

## Examples

Creates a new dataframe and shows it in the dataframe list
```shell
> let test = ([[a b];[1 2] [3 4]] | dfr to-df);
    dfr list
```
