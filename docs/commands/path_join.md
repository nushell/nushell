---
title: path join
layout: command
version: 0.59.1
---

Join a structured path or a list of path parts.

## Signature

```> path join (append) --columns```

## Parameters

 -  `append`: Path to append to the input
 -  `--columns {table}`: Optionally operate by column path

## Examples

Append a filename to a path
```shell
> '/home/viking' | path join spam.txt
```

Append a filename to a path inside a column
```shell
> ls | path join spam.txt -c [ name ]
```

Join a list of parts into a path
```shell
> [ '/' 'home' 'viking' 'spam.txt' ] | path join
```

Join a structured path into a path
```shell
> [[ parent stem extension ]; [ '/home/viking' 'spam' 'txt' ]] | path join
```
