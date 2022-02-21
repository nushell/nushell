---
title: url query
layout: command
version: 0.59.0
---

gets the query of a url

## Signature

```> url query ...rest```

## Parameters

 -  `...rest`: optionally operate by cell path

## Examples

Get query of a url
```shell
> echo 'http://www.example.com/?foo=bar&baz=quux' | url query
```

No query gives the empty string
```shell
> echo 'http://www.example.com/' | url query
```
