---
title: hash base64
layout: command
version: 0.59.1
---

base64 encode or decode a value

## Signature

```> hash base64 ...rest --character-set --encode --decode```

## Parameters

 -  `...rest`: optionally base64 encode / decode data by column paths
 -  `--character-set {string}`: specify the character rules for encoding the input.
	Valid values are 'standard', 'standard-no-padding', 'url-safe', 'url-safe-no-padding','binhex', 'bcrypt', 'crypt'
 -  `--encode`: encode the input as base64. This is the default behavior if not specified.
 -  `--decode`: decode the input from base64

## Examples

Base64 encode a string with default settings
```shell
> echo 'username:password' | hash base64
```

Base64 encode a string with the binhex character set
```shell
> echo 'username:password' | hash base64 --character-set binhex --encode
```

Base64 decode a value
```shell
> echo 'dXNlcm5hbWU6cGFzc3dvcmQ=' | hash base64 --decode
```
