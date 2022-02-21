---
title: get
layout: command
version: 0.59.0
---

Extract data using a cell path.

## Signature

```> get (cell_path) ...rest --ignore-errors```

## Parameters

 -  `cell_path`: the cell path to the data
 -  `...rest`: additional cell paths
 -  `--ignore-errors`: return nothing if path can't be found

## Examples

Extract the name of files as a list
```shell
> ls | get name
```

Extract the name of the 3rd entry of a file list
```shell
> ls | get name.2
```

Extract the name of the 3rd entry of a file list (alternative)
```shell
> ls | get 2.name
```

Extract the cpu list from the sys information record
```shell
> sys | get cpu
```
