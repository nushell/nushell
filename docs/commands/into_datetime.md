---
title: into datetime
layout: command
version: 0.59.1
---

converts text into datetime

## Signature

```> into datetime ...rest --list --timezone --offset --format```

## Parameters

 -  `...rest`: optionally convert text into datetime by column paths
 -  `--list`: lists strftime cheatsheet
 -  `--timezone {string}`: Specify timezone if the input is timestamp, like 'UTC/u' or 'LOCAL/l'
 -  `--offset {int}`: Specify timezone by offset if the input is timestamp, like '+8', '-4', prior than timezone
 -  `--format {string}`: Specify date and time formatting

## Examples

Convert to datetime
```shell
> '16.11.1984 8:00 am +0000' | into datetime
```

Convert to datetime
```shell
> '2020-08-04T16:39:18+00:00' | into datetime
```

Convert to datetime using a custom format
```shell
> '20200904_163918+0000' | into datetime -f '%Y%m%d_%H%M%S%z'
```

Convert timestamp (no larger than 8e+12) to datetime using a specified timezone
```shell
> '1614434140' | into datetime -z 'UTC'
```

Convert timestamp (no larger than 8e+12) to datetime using a specified timezone offset (between -12 and 12)
```shell
> '1614434140' | into datetime -o +9
```
