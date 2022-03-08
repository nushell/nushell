---
title: empty?
layout: command
version: 0.59.1
---

Check for empty values.

## Signature

```> empty? ...rest --block```

## Parameters

 -  `...rest`: the names of the columns to check emptiness
 -  `--block {block}`: an optional block to replace if empty

## Examples

Check if a value is empty
```shell
> '' | empty?
```

more than one column
```shell
> [[meal size]; [arepa small] [taco '']] | empty? meal size
```

use a block if setting the empty cell contents is wanted
```shell
> [[2020/04/16 2020/07/10 2020/11/16]; ['' [27] [37]]] | empty? 2020/04/16 -b { |_| [33 37] }
```
