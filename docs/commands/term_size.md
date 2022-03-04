---
title: term size
layout: command
version: 0.59.1
---

Returns the terminal size

## Signature

```> term size --columns --rows```

## Parameters

 -  `--columns`: Report only the width of the terminal
 -  `--rows`: Report only the height of the terminal

## Examples

Return the width height of the terminal
```shell
> term size
```

Return the width (columns) of the terminal
```shell
> term size -c
```

Return the height (rows) of the terminal
```shell
> term size -r
```
