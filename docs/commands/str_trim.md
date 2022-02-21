---
title: str trim
layout: command
version: 0.59.0
---

trims text

## Signature

```> str trim ...rest --char --left --right --all --both --format```

## Parameters

 -  `...rest`: optionally trim text by column paths
 -  `--char {string}`: character to trim (default: whitespace)
 -  `--left`: trims characters only from the beginning of the string (default: whitespace)
 -  `--right`: trims characters only from the end of the string (default: whitespace)
 -  `--all`: trims all characters from both sides of the string *and* in the middle (default: whitespace)
 -  `--both`: trims all characters from left and right side of the string (default: whitespace)
 -  `--format`: trims spaces replacing multiple characters with singles in the middle (default: whitespace)

## Examples

Trim whitespace
```shell
> 'Nu shell ' | str trim
```

Trim a specific character
```shell
> '=== Nu shell ===' | str trim -c '=' | str trim
```

Trim all characters
```shell
> ' Nu   shell ' | str trim -a
```

Trim whitespace from the beginning of string
```shell
> ' Nu shell ' | str trim -l
```

Trim a specific character
```shell
> '=== Nu shell ===' | str trim -c '='
```

Trim whitespace from the end of string
```shell
> ' Nu shell ' | str trim -r
```

Trim a specific character
```shell
> '=== Nu shell ===' | str trim -r -c '='
```
