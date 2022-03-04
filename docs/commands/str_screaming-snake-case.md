---
title: str screaming-snake-case
layout: command
version: 0.59.1
---

converts a string to SCREAMING_SNAKE_CASE

## Signature

```> str screaming-snake-case ...rest```

## Parameters

 -  `...rest`: optionally convert text to SCREAMING_SNAKE_CASE by column paths

## Examples

convert a string to camelCase
```shell
>  "NuShell" | str screaming-snake-case
```

convert a string to camelCase
```shell
>  "this_is_the_second_case" | str screaming-snake-case
```

convert a string to camelCase
```shell
> "this-is-the-first-case" | str screaming-snake-case
```

convert a column from a table to SCREAMING_SNAKE_CASE
```shell
> [[lang, gems]; [nu_test, 100]] | str screaming-snake-case lang
```
