# empty?

Check for empty values. Pass the column names to check emptiness. Optionally pass a block as the last parameter if setting contents to empty columns is wanted.

## Examples

Check if a value is empty
```shell
> echo '' | empty?
true
```

Given the following meals
```shell
> echo [[meal size]; [arepa small] [taco '']]
═══╦═══════╦═══════
 # ║ meal  ║ size
═══╬═══════╬═══════
 0 ║ arepa ║ small
 1 ║ taco  ║
═══╩═══════╩═══════
```

Show the empty contents
```shell
> echo [[meal size]; [arepa small] [taco '']] | empty? meal size
═══╦═══════╦═══════
 # ║ meal  ║ size
═══╬═══════╬═══════
 0 ║ false ║ false
 1 ║ false ║ true
═══╩═══════╩═══════
```

Let's assume we have a report of totals per day. For simplicity we show just for three days `2020/04/16`, `2020/07/10`, and `2020/11/16`. Like so
```shell
> echo [[2020/04/16 2020/07/10 2020/11/16]; ['' 27 37]]
═══╦════════════╦════════════╦════════════
 # ║ 2020/04/16 ║ 2020/07/10 ║ 2020/11/16
═══╬════════════╬════════════╬════════════
 0 ║            ║         27 ║         37
═══╩════════════╩════════════╩════════════
```

In the future, the report now has many totals logged per day. In this example, we have 1 total for the day `2020/07/10` and `2020/11/16` like so
```shell
> echo [[2020/04/16 2020/07/10 2020/11/16]; ['' [27] [37]]]
═══╦════════════╦════════════════╦════════════════
 # ║ 2020/04/16 ║ 2020/07/10     ║ 2020/11/16
═══╬════════════╬════════════════╬════════════════
 0 ║            ║ [table 1 rows] ║ [table 1 rows]
═══╩════════════╩════════════════╩════════════════
```

We want to add two totals (numbers `33` and `37`) for the day `2020/04/16`

Set a table with two numbers for the empty column
```shell
> echo [[2020/04/16 2020/07/10 2020/11/16]; ['' [27] [37]]] | empty? 2020/04/16 { = [33 37] }
═══╦════════════════╦════════════════╦════════════════
 # ║ 2020/04/16     ║ 2020/07/10     ║ 2020/11/16
═══╬════════════════╬════════════════╬════════════════
 0 ║ [table 2 rows] ║ [table 1 rows] ║ [table 1 rows]
═══╩════════════════╩════════════════╩════════════════
```

Checking all the numbers
```shell
> echo [[2020/04/16 2020/07/10 2020/11/16]; ['' [27] [37]]] | empty? 2020/04/16 { = [33 37] } | pivot _ totals | get totals
═══╦════
 0 ║ 33
 1 ║ 37
 2 ║ 27
 3 ║ 37
═══╩════
```