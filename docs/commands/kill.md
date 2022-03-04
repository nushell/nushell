---
title: kill
layout: command
version: 0.59.1
---

Kill a process using the process id.

## Signature

```> kill (pid) ...rest --force --quiet --signal```

## Parameters

 -  `pid`: process id of process that is to be killed
 -  `...rest`: rest of processes to kill
 -  `--force`: forcefully kill the process
 -  `--quiet`: won't print anything to the console
 -  `--signal {int}`: signal decimal number to be sent instead of the default 15 (unsupported on Windows)

## Examples

Kill the pid using the most memory
```shell
> ps | sort-by mem | last | kill $in.pid
```

Force kill a given pid
```shell
> kill --force 12345
```

Send INT signal
```shell
> kill -s 2 12345
```
