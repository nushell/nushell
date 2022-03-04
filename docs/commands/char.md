---
title: char
layout: command
version: 0.59.1
---

Output special characters (e.g., 'newline').

## Signature

```> char (character) ...rest --list --unicode```

## Parameters

 -  `character`: the name of the character to output
 -  `...rest`: multiple Unicode bytes
 -  `--list`: List all supported character names
 -  `--unicode`: Unicode string i.e. 1f378

## Examples

Output newline
```shell
> char newline
```

Output prompt character, newline and a hamburger character
```shell
> echo [(char prompt) (char newline) (char hamburger)] | str collect
```

Output Unicode character
```shell
> char -u 1f378
```

Output multi-byte Unicode character
```shell
> char -u 1F468 200D 1F466 200D 1F466
```
