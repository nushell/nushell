---
title: str snake-case
layout: command
version: 0.59.0
---

converts a string to snake_case

## Signature

```> str snake-case ...rest```

## Parameters

 -  `...rest`: optionally convert text to snake_case by column paths

## Examples

convert a string to camelCase
```shell
>  "NuShell" | str snake-case
```

convert a string to camelCase
```shell
>  "this_is_the_second_case" | str snake-case
```

convert a string to camelCase
```shell
> "this-is-the-first-case" | str snake-case
```

convert a column from a table to snake-case
```shell
> [[lang, gems]; [nuTest, 100]] | str snake-case lang
```
