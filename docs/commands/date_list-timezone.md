---
title: date list-timezone
layout: command
version: 0.59.1
---

List supported time zones.

## Signature

```> date list-timezone ```

## Examples

Show timezone(s) that contains 'Shanghai'
```shell
> date list-timezone | where timezone =~ Shanghai
```
