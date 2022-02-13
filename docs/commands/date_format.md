---
title: date format
layout: command
version: 0.59.0
---

Format a given date using the given format string.

## Signature

```> date format (format string)```

## Parameters

 -  `format string`: the desired date format

## Examples

Format a given date using the given format string.
```shell
date format '%Y-%m-%d'
```

Format a given date using the given format string.
```shell
date format "%Y-%m-%d %H:%M:%S"
```

Format a given date using the given format string.
```shell
"2021-10-22 20:00:12 +01:00" | date format "%Y-%m-%d"
```

