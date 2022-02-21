---
title: url path
layout: command
version: 0.59.0
---

gets the path of a url

## Signature

```> url path ...rest```

## Parameters

 -  `...rest`: optionally operate by cell path

## Examples

Get path of a url
```shell
> echo 'http://www.example.com/foo/bar' | url path
```

A trailing slash will be reflected in the path
```shell
> echo 'http://www.example.com' | url path
```
