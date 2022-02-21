---
title: url scheme
layout: command
version: 0.59.0
---

gets the scheme (eg http, file) of a url

## Signature

```> url scheme ...rest```

## Parameters

 -  `...rest`: optionally operate by cell path

## Examples

Get scheme of a url
```shell
> echo 'http://www.example.com' | url scheme
```

You get an empty string if there is no scheme
```shell
> echo 'test' | url scheme
```
