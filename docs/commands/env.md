---
title: env
layout: command
version: 0.59.0
---

Display current environment variables

## Signature

```> env ```

## Examples

Display current path environment variable
```shell
> env | where name == PATH
```

Check whether the env variable `MY_ENV_ABC` exists
```shell
> env | any? name == MY_ENV_ABC
```

Another way to check whether the env variable `PATH` exists
```shell
> 'PATH' in (env).name
```
