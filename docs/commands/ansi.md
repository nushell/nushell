---
title: ansi
layout: command
version: 0.59.0
---

Output ANSI codes to change color.

## Signature

```> ansi (code) --escape --osc --list```

## Parameters

 -  `code`: the name of the code to use like 'green' or 'reset' to reset the color
 -  `--escape`: escape sequence without the escape character(s)
 -  `--osc`: operating system command (ocs) escape sequence without the escape character(s)
 -  `--list`: list available ansi code names

## Examples

Change color to green
```shell
> ansi green
```

Reset the color
```shell
> ansi reset
```

Use ansi to color text (rb = red bold, gb = green bold, pb = purple bold)
```shell
> echo [(ansi rb) Hello " " (ansi gb) Nu " " (ansi pb) World (ansi reset)] | str collect
```

Use ansi to color text (italic bright yellow on red 'Hello' with green bold 'Nu' and purble bold 'World')
```shell
> echo [(ansi -e '3;93;41m') Hello (ansi reset) " " (ansi gb) Nu " " (ansi pb) World (ansi reset)] | str collect
```

Use ansi to color text with a style (blue on red in bold)
```shell
> $"(ansi -e { fg: '#0000ff' bg: '#ff0000' attr: b })Hello Nu World(ansi reset)"
```
