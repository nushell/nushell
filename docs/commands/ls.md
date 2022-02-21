---
title: ls
layout: command
version: 0.59.0
---

List the files in a directory.

## Signature

```> ls (pattern) --all --long --short-names --full-paths --du```

## Parameters

 -  `pattern`: the glob pattern to use
 -  `--all`: Show hidden files
 -  `--long`: List all available columns for each entry
 -  `--short-names`: Only print the file names and not the path
 -  `--full-paths`: display paths as absolute paths
 -  `--du`: Display the apparent directory size in place of the directory metadata size

## Examples

List all files in the current directory
```shell
> ls
```

List all files in a subdirectory
```shell
> ls subdir
```

List all rust files
```shell
> ls *.rs
```
