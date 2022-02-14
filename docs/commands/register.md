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

