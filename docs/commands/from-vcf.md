# from vcf

Parse text as `.vcf` and create table.

Syntax: `from vcf`

## Examples

Suppose contacts.txt is a text file that is formatted like a `.vcf` (vCard) file:

```shell
> open contacts.txt
BEGIN:VCARD
VERSION:3.0
FN:John Doe
N:Doe;John;;;
EMAIL;TYPE=INTERNET:john.doe99@gmail.com
...
```

Pass the output of the `open` command to `from vcf` to get a correctly formatted table:

```shell
> open contacts.txt | from vcf
─────┬─────────────────
 #   │ properties
─────┼─────────────────
   0 │ [table 8 rows]
```

```shell
> open contacts.txt | from vcf | get properties | where $it.name == "FN" | select value
─────┬──────────────────────
 #   │
─────┼──────────────────────
   0 │ John Doe
```
