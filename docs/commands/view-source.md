---
title: view-source
layout: command
version: 0.59.1
---

View a block, module, or a definition

## Signature

```> view-source (item)```

## Parameters

 -  `item`: name or block to view

## Examples

View the source of a code block
```shell
> let abc = { echo 'hi' }; view-source $abc
```

View the source of a custom command
```shell
> def hi [] { echo 'Hi!' }; view-source hi
```

View the source of a custom command, which participates in the caller environment
```shell
> def-env foo [] { let-env BAR = 'BAZ' }; view-source foo
```

View the source of a module
```shell
> module mod-foo { export env FOO_ENV { 'BAZ' } }; view-source mod-foo
```
