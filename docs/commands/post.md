---
title: post
layout: command
version: 0.59.1
---

Post a body to a URL (HTTP POST operation).

## Signature

```> post (path) (body) --user --password --content-type --content-length --headers --raw --insecure```

## Parameters

 -  `path`: the URL to post to
 -  `body`: the contents of the post body
 -  `--user {any}`: the username when authenticating
 -  `--password {any}`: the password when authenticating
 -  `--content-type {any}`: the MIME type of content to post
 -  `--content-length {any}`: the length of the content being posted
 -  `--headers {any}`: custom headers you want to add
 -  `--raw`: return values as a string instead of a table
 -  `--insecure`: allow insecure server connections when using SSL

## Examples

Post content to url.com
```shell
> post url.com 'body'
```

Post content to url.com, with username and password
```shell
> post -u myuser -p mypass url.com 'body'
```

Post content to url.com, with custom header
```shell
> post -H [my-header-key my-header-value] url.com
```

Post content to url.com with a json body
```shell
> post -t application/json url.com { field: value }
```
