# sum 
This command allows you to calculate the sum of values in a column.  

## Examples 
To get the sum of the file sizes in a directory, simply pipe the size column from the ls command to the sum command.

```shell
> ls | get size | sum
━━━━━━━━━
 value
━━━━━━━━━
 51.0 MB
━━━━━━━━━
```

To get the sum of the characters that make up your  present working directory.
```shell
> pwd | split-row / | size | get chars | sum
━━━━━━━━━
 <value>
━━━━━━━━━
21
━━━━━━━━━
```

Note that sum only works for integer and byte values. If the shell doesn't recognize the values in a column as one of those types, it will return an error.
One way to solve this is to convert each row to an integer when possible and then pipe the result to `sum`

```shell
> open tests/fixtures/formats/caco3_plastics.csv | get tariff_item | sum
error: Unrecognized type in stream: Primitive(String("2509000000"))
- shell:1:0
1 | open tests/fixtures/formats/caco3_plastics.csv | get tariff_item | sum
  | ^^^^ source
```

```shell
> open tests/fixtures/formats/caco3_plastics.csv | get tariff_item | str --to-int | sum
━━━━━━━━━━━━━
 <value>
─────────────
 29154639996
━━━━━━━━━━━━━
```
