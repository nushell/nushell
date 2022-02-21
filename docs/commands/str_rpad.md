---
title: str rpad
layout: command
version: 0.59.0
---

pad a string with a character a certain length

## Signature

```> str rpad ...rest --length --character```

## Parameters

 -  `...rest`: optionally check if string contains pattern by column paths
 -  `--length {int}`: length to pad to
 -  `--character {string}`: character to pad with

## Examples

Right pad a string with a character a number of places
```shell
> 'nushell' | str rpad -l 10 -c '*'
```

Right pad a string with a character a number of places
```shell
> '123' | str rpad -l 10 -c '0'
```

Use rpad to truncate a string
```shell
> '123456789' | str rpad -l 3 -c '0'
```

Use rpad to pad Unicode
```shell
> '▉' | str rpad -l 10 -c '▉'
```
