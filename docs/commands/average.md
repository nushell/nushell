# average 
This command allows you to calculate the average of values in a column.  

## Examples 
To get the average of the file sizes in a directory, simply pipe the size column from the ls command to the average command.

```shell
> ls | get size | average
━━━━━━━━━
 <value>
━━━━━━━━━
2282.727272727273
━━━━━━━━━
```

```shell
> pwd | split-row / | size | get chars | average
━━━━━━━━━
 <value>
━━━━━━━━━
5.250000000000000
━━━━━━━━━
```

Note that average only works for integer and byte values. If the shell doesn't recognize the values in a column as one of those types, it will return an error.
One way to solve this is to convert each row to an integer when possible and then pipe the result to `average`

```shell
> open tests/fixtures/formats/caco3_plastics.csv | get tariff_item | average
error: Unrecognized type in stream: Primitive(String("2509000000"))
- shell:1:0
1 | open tests/fixtures/formats/caco3_plastics.csv | get tariff_item | average
  | ^^^^ source
```

```shell
> open tests/fixtures/formats/caco3_plastics.csv | get tariff_item | str --to-int | average
━━━━━━━━━━━━━━━━━━━
 <value>
───────────────────
 3239404444.000000
━━━━━━━━━━━━━━━━━━━
```


