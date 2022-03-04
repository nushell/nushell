---
title: fetch
layout: command
version: 0.59.1
---

Fetch the contents from a URL (HTTP GET operation).

## Signature

```> fetch (URL) --user --password --timeout --headers --raw```

## Parameters

 -  `URL`: the URL to fetch the contents from
 -  `--user {any}`: the username when authenticating
 -  `--password {any}`: the password when authenticating
 -  `--timeout {int}`: timeout period in seconds
 -  `--headers {any}`: custom headers you want to add
 -  `--raw`: fetch contents as text rather than a table

## Examples

Fetch content from url.com
```shell
> fetch url.com
```

Fetch content from url.com, with username and password
```shell
> fetch -u myuser -p mypass url.com
```

Fetch content from url.com, with custom header
```shell
> fetch -H [my-header-key my-header-value] url.com
```
