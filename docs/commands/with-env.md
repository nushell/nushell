---
title: with-env
layout: command
version: 0.59.1
---

Runs a block with an environment variable set.

## Signature

```> with-env (variable) (block)```

## Parameters

 -  `variable`: the environment variable to temporarily set
 -  `block`: the block to run once the variable is set

## Examples

Set the MYENV environment variable
```shell
> with-env [MYENV "my env value"] { $env.MYENV }
```

Set by primitive value list
```shell
> with-env [X Y W Z] { $env.X }
```

Set by single row table
```shell
> with-env [[X W]; [Y Z]] { $env.W }
```

Set by row(e.g. `open x.json` or `from json`)
```shell
> echo '{"X":"Y","W":"Z"}'|from json|with-env $in { echo $env.X $env.W }
```
