---
title: use
layout: command
version: 0.59.0
---

Use definitions from a module

## Signature

```> use (pattern)```

## Parameters

 -  `pattern`: import pattern

## Examples

Define a custom command in a module and call it
```shell
> module spam { export def foo [] { "foo" } }; use spam foo; foo
```

Define an environment variable in a module and evaluate it
```shell
> module foo { export env FOO_ENV { "BAZ" } }; use foo FOO_ENV; $env.FOO_ENV
```

Define a custom command that participates in the environment in a module and call it
```shell
> module foo { export def-env bar [] { let-env FOO_BAR = "BAZ" } }; use foo bar; bar; $env.FOO_BAR
```
