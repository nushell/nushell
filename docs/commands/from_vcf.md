---
title: from vcf
layout: command
version: 0.59.0
---

Parse text as .vcf and create table.

## Signature

```> from vcf ```

## Examples

Converts ics formatted string to table
```shell
> 'BEGIN:VCARD
N:Foo
FN:Bar
EMAIL:foo@bar.com
END:VCARD' | from vcf
```
