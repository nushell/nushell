---
title: from eml
layout: command
version: 0.59.0
---

Parse text as .eml and create table.

## Signature

```> from eml --preview-body```

## Parameters

 -  `--preview-body {int}`: How many bytes of the body to preview

## Examples

Convert eml structured data into table
```shell
> 'From: test@email.com
Subject: Welcome
To: someone@somewhere.com

Test' | from eml
```

Convert eml structured data into table
```shell
> 'From: test@email.com
Subject: Welcome
To: someone@somewhere.com

Test' | from eml -b 1
```
