---
title: date to-timezone
layout: command
version: 0.59.1
---

Convert a date to a given time zone.

## Signature

```> date to-timezone (time zone)```

## Parameters

 -  `time zone`: time zone description

## Examples

Get the current date in UTC+05:00
```shell
> date now | date to-timezone +0500
```

Get the current local date
```shell
> date now | date to-timezone local
```

Get the current date in Hawaii
```shell
> date now | date to-timezone US/Hawaii
```

Get the current date in Hawaii
```shell
> "2020-10-10 10:00:00 +02:00" | date to-timezone "+0500"
```
