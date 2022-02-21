---
title: find
layout: command
version: 0.59.0
---

Searches terms in the input or for elements of the input that satisfies the predicate.

## Signature

```> find ...rest --predicate```

## Parameters

 -  `...rest`: terms to search
 -  `--predicate {block}`: the predicate to satisfy

## Examples

Search for multiple terms in a command output
```shell
> ls | find toml md sh
```

Search for a term in a string
```shell
> echo Cargo.toml | find toml
```

Search a number or a file size in a list of numbers
```shell
> [1 5 3kb 4 3Mb] | find 5 3kb
```

Search a char in a list of string
```shell
> [moe larry curly] | find l
```

Find the first odd value
```shell
> echo [2 4 3 6 5 8] | find --predicate { |it| ($it mod 2) == 1 }
```

Find if a service is not running
```shell
> echo [[version patch]; [0.1.0 $false] [0.1.1 $true] [0.2.0 $false]] | find -p { |it| $it.patch }
```
