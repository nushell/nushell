---
title: str find-replace
layout: command
version: 0.59.0
---

finds and replaces text

## Signature

```> str find-replace (find) (replace) ...rest --all```

## Parameters

 -  `find`: the pattern to find
 -  `replace`: the replacement pattern
 -  `...rest`: optionally find and replace text by column paths
 -  `--all`: replace all occurrences of find string

## Examples

Find and replace contents with capture group
```shell
> 'my_library.rb' | str find-replace '(.+).rb' '$1.nu'
```

Find and replace all occurrences of find string
```shell
> 'abc abc abc' | str find-replace -a 'b' 'z'
```

Find and replace all occurrences of find string in table
```shell
> [[ColA ColB ColC]; [abc abc ads]] | str find-replace -a 'b' 'z' ColA ColC
```
