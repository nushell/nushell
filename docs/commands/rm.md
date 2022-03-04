---
title: rm
layout: command
version: 0.59.1
---

Remove file(s).

## Signature

```> rm ...rest --trash --permanent --recursive --force --quiet```

## Parameters

 -  `...rest`: the file path(s) to remove
 -  `--trash`: use the platform's recycle bin instead of permanently deleting
 -  `--permanent`: don't use recycle bin, delete permanently
 -  `--recursive`: delete subdirectories recursively
 -  `--force`: suppress error when no file
 -  `--quiet`: suppress output showing files deleted

## Examples

Delete or move a file to the system trash (depending on 'rm_always_trash' config option)
```shell
> rm file.txt
```

Move a file to the system trash
```shell
> rm --trash file.txt
```

Delete a file permanently
```shell
> rm --permanent file.txt
```

Delete a file, and suppress errors if no file is found
```shell
> rm --force file.txt
```
