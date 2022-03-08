---
title: reduce
layout: command
version: 0.59.1
---

Aggregate a list table to a single value using an accumulator block.

## Signature

```> reduce (block) --fold --numbered```

## Parameters

 -  `block`: reducing function
 -  `--fold {any}`: reduce with initial value
 -  `--numbered`: iterate with an index

## Examples

Sum values of a list (same as 'math sum')
```shell
> [ 1 2 3 4 ] | reduce {|it, acc| $it + $acc }
```

Sum values with a starting value (fold)
```shell
> [ 1 2 3 4 ] | reduce -f 10 {|it, acc| $acc + $it }
```

Replace selected characters in a string with 'X'
```shell
> [ i o t ] | reduce -f "Arthur, King of the Britons" {|it, acc| $acc | str find-replace -a $it "X" }
```

Find the longest string and its index
```shell
> [ one longest three bar ] | reduce -n { |it, acc|
        if ($it.item | str length) > ($acc | str length) {
            $it.item
        } else {
            $acc
        }
    }
```
