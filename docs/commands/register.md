---
title: register
layout: command
version: 0.59.0
---

Register a plugin

## Signature

```> register (plugin) (signature) --encoding --shell```

## Parameters

 -  `plugin`: path of executable for plugin
 -  `signature`: Block with signature description as json object
 -  `--encoding {string}`: Encoding used to communicate with plugin. Options: [capnp, json]
 -  `--shell {path}`: path of shell used to run plugin (cmd, sh, python, etc)

## Examples

Register `nu_plugin_extra_query` plugin from ~/.cargo/bin/ dir
```shell
> register -e capnp ~/.cargo/bin/nu_plugin_extra_query
```

Register `nu_plugin_extra_query` plugin from `nu -c`(plugin will be available in that nu session only)
```shell
> let plugin = ((which nu).path.0 | path dirname | path join 'nu_plugin_extra_query'); nu -c $'register -e capnp ($plugin); version'
```
