---
title: str contains
layout: command
version: 0.59.0
---

Checks if string contains pattern

## Signature

```> str contains (pattern) ...rest --insensitive```

## Parameters

 -  `pattern`: the pattern to find
 -  `...rest`: optionally check if string contains pattern by column paths
 -  `--insensitive`: search is case insensitive

## Examples

Check if string contains pattern
```shell
> 'my_library.rb' | str contains '.rb'
```

Check if string contains pattern case insensitive
```shell
> 'my_library.rb' | str contains -i '.RB'
```

Check if string contains pattern in a table
```shell
>  [[ColA ColB]; [test 100]] | str contains 'e' ColA
```

Check if string contains pattern in a table
```shell
>  [[ColA ColB]; [test 100]] | str contains -i 'E' ColA
```

Check if string contains pattern in a table
```shell
>  [[ColA ColB]; [test hello]] | str contains 'e' ColA ColB
```

Check if string contains pattern
```shell
> 'hello' | str contains 'banana'
```
