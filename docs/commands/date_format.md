---
title: date format
layout: command
version: 0.59.1
---

Format a given date using a format string.

## Signature

```> date format (format string) --list```

## Parameters

 -  `format string`: the desired date format
 -  `--list`: lists strftime cheatsheet

## Examples

Format a given date using the default format (RFC 2822).
```shell
> "2021-10-22 20:00:12 +01:00" | date format
```

Format a given date using a given format string.
```shell
> date format '%Y-%m-%d'
```

Format a given date using a given format string.
```shell
> date format "%Y-%m-%d %H:%M:%S"
```

Format a given date using a given format string.
```shell
> "2021-10-22 20:00:12 +01:00" | date format "%Y-%m-%d"
```
