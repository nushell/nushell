---
title: from ics
layout: command
version: 0.59.0
---

Parse text as .ics and create table.

## Signature

```> from ics ```

## Examples

Converts ics formatted string to table
```shell
> 'BEGIN:VCALENDAR
END:VCALENDAR' | from ics
```
