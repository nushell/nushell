---
title: export
layout: command
version: 0.59.1
---

Export custom commands or environment variables from a module.

## Signature

```> export ```

## Examples

Export a definition from a module
```shell
> module utils { export def my-command [] { "hello" } }; use utils my-command; my-command
```
