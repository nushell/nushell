---
title: from xml
layout: command
version: 0.59.1
---

Parse text as .xml and create table.

## Signature

```> from xml ```

## Examples

Converts xml formatted string to table
```shell
> '<?xml version="1.0" encoding="UTF-8"?>
<note>
  <remember>Event</remember>
</note>' | from xml
```
