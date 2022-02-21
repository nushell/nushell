---
title: ansi gradient
layout: command
version: 0.59.0
---

draw text with a provided start and end code making a gradient

## Signature

```> ansi gradient ...column path --fgstart --fgend --bgstart --bgend```

## Parameters

 -  `...column path`: optionally, draw gradients using text from column paths
 -  `--fgstart {string}`: foreground gradient start color in hex (0x123456)
 -  `--fgend {string}`: foreground gradient end color in hex
 -  `--bgstart {string}`: background gradient start color in hex
 -  `--bgend {string}`: background gradient end color in hex

## Examples

draw text in a gradient with foreground start and end colors
```shell
> echo 'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart 0x40c9ff --fgend 0xe81cff
```

draw text in a gradient with foreground start and end colors and background start and end colors
```shell
> echo 'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart 0x40c9ff --fgend 0xe81cff --bgstart 0xe81cff --bgend 0x40c9ff
```

draw text in a gradient by specifying foreground start color - end color is assumed to be black
```shell
> echo 'Hello, Nushell! This is a gradient.' | ansi gradient --fgstart 0x40c9ff
```

draw text in a gradient by specifying foreground end color - start color is assumed to be black
```shell
> echo 'Hello, Nushell! This is a gradient.' | ansi gradient --fgend 0xe81cff
```
