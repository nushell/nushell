---
title: rm
layout: command
version: 0.59.0
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
 -  `--quiet`: supress output showing files deleted

