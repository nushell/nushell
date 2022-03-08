---
title: str lpad
layout: command
version: 0.59.1
---

pad a string with a character a certain length

## Signature

```> str lpad ...rest --length --character```

## Parameters

 -  `...rest`: optionally check if string contains pattern by column paths
 -  `--length {int}`: length to pad to
 -  `--character {string}`: character to pad with

## Examples

Left pad a string with a character a number of places
```shell
> 'nushell' | str lpad -l 10 -c '*'
```

Left pad a string with a character a number of places
```shell
> '123' | str lpad -l 10 -c '0'
```

Use lpad to truncate a string
```shell
> '123456789' | str lpad -l 3 -c '0'
```

Use lpad to pad Unicode
```shell
> '▉' | str lpad -l 10 -c '▉'
```
