---
title: source
layout: command
version: 0.59.1
---

Runs a script file in the current context.

## Signature

```> source (filename)```

## Parameters

 -  `filename`: the filepath to the script file to source

## Examples

Runs foo.nu in the current context
```shell
> source foo.nu
```

Runs foo.nu in current context and call the command defined, suppose foo.nu has content: `def say-hi [] { echo 'Hi!' }`
```shell
> source ./foo.nu; say-hi
```

Runs foo.nu in current context and call the `main` command automatically, suppose foo.nu has content: `def main [] { echo 'Hi!' }`
```shell
> source ./foo.nu
```
