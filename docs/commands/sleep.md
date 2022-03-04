---
title: sleep
layout: command
version: 0.59.1
---

Delay for a specified amount of time.

## Signature

```> sleep (duration) ...rest```

## Parameters

 -  `duration`: time to sleep
 -  `...rest`: additional time

## Examples

Sleep for 1sec
```shell
> sleep 1sec
```

Sleep for 3sec
```shell
> sleep 1sec 1sec 1sec
```

Send output after 1sec
```shell
> sleep 1sec; echo done
```
