---
title: date now
layout: command
version: 0.59.0
---

Get the current date.

## Signature

```> date now ```

## Examples

Get the current date and display it in a given format string.
```shell
> date now | date format "%Y-%m-%d %H:%M:%S"
```

Get the time duration from 2019-04-30 to now
```shell
> (date now) - 2019-05-01
```

Get the time duration since a more accurate time
```shell
> (date now) - 2019-05-01T04:12:05.20+08:00
```

Get current time in full RFC3339 format with timezone
```shell
> date now | debug
```
