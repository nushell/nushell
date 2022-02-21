---
title: path basename
layout: command
version: 0.59.0
---

Get the final component of a path

## Signature

```> path basename --columns --replace```

## Parameters

 -  `--columns {table}`: Optionally operate by column path
 -  `--replace {string}`: Return original path with basename replaced by this string

## Examples

Get basename of a path
```shell
> '/home/joe/test.txt' | path basename
```

Get basename of a path by column
```shell
> [[name];[/home/joe]] | path basename -c [ name ]
```

Replace basename of a path
```shell
> '/home/joe/test.txt' | path basename -r 'spam.png'
```
