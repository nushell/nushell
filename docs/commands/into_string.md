---
title: into string
layout: command
version: 0.59.0
---

Convert value to string

## Signature

```> into string ...rest --decimals```

## Parameters

 -  `...rest`: column paths to convert to string (for table input)
 -  `--decimals {int}`: decimal digits to which to round

## Examples

convert decimal to string and round to nearest integer
```shell
> 1.7 | into string -d 0
```

convert decimal to string
```shell
> 1.7 | into string -d 1
```

convert decimal to string and limit to 2 decimals
```shell
> 1.734 | into string -d 2
```

try to convert decimal to string and provide negative decimal points
```shell
> 1.734 | into string -d -2
```

convert decimal to string
```shell
> 4.3 | into string
```

convert string to string
```shell
> '1234' | into string
```

convert boolean to string
```shell
> $true | into string
```

convert date to string
```shell
> date now | into string
```

convert filepath to string
```shell
> ls Cargo.toml | get name | into string
```

convert filesize to string
```shell
> ls Cargo.toml | get size | into string
```
