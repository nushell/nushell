---
title: open
layout: command
version: 0.59.1
---

Opens a file.

## Signature

```> open (filename) --raw```

## Parameters

 -  `filename`: the filename to use
 -  `--raw`: open file as raw binary

## Examples

Open a file, with structure (based on file extension)
```shell
> open myfile.json
```

Open a file, as raw bytes
```shell
> open myfile.json --raw
```

Open a file, using the input to get filename
```shell
> echo 'myfile.txt' | open
```
