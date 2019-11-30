# from-xml

Parse text as `.xml` and create table. Use this when nushell cannot dertermine the input file extension.

Syntax: `from-xml`

## Examples

Let's say we've got a file in `xml` format but the file extension is different so Nu can't auto-format it:

```shell
> open world.txt
<?xml version="1.0" encoding="utf-8"?>
<world>
    <continent>Africa</continent>
    <continent>Antarctica</continent>
    <continent>Asia</continent>
    <continent>Australia</continent>
    <continent>Europe</continent>
    <continent>North America</continent>
    <continent>South America</continent>
</world>
```

We can use `from-xml` to read the input like a `xml` file:

```shell
> open world.txt | from-xml
━━━━━━━━━━━━━━━━
 world
────────────────
 [table 7 rows]
━━━━━━━━━━━━━━━━
```