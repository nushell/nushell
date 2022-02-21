---
title: into int
layout: command
version: 0.59.0
---

Convert value to integer

## Signature

```> into int ...rest --radix```

## Parameters

 -  `...rest`: column paths to convert to int (for table input)
 -  `--radix {number}`: radix of integer

## Examples

Convert string to integer in table
```shell
> echo [[num]; ['-5'] [4] [1.5]] | into int num
```

Convert string to integer
```shell
> '2' | into int
```

Convert decimal to integer
```shell
> 5.9 | into int
```

Convert decimal string to integer
```shell
> '5.9' | into int
```

Convert file size to integer
```shell
> 4KB | into int
```

Convert bool to integer
```shell
> [$false, $true] | into int
```

Convert to integer from binary
```shell
> '1101' | into int -r 2
```

Convert to integer from hex
```shell
> 'FF' |  into int -r 16
```
