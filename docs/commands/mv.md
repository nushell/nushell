---
title: mv
layout: command
version: 0.59.1
---

Move files or directories.

## Signature

```> mv (source) (destination)```

## Parameters

 -  `source`: the location to move files/directories from
 -  `destination`: the location to move files/directories to

## Examples

Rename a file
```shell
> mv before.txt after.txt
```

Move a file into a directory
```shell
> mv test.txt my/subdirectory
```

Move many files into a directory
```shell
> mv *.txt my/subdirectory
```
