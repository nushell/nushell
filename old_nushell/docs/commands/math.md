# math

Mathematical functions that generally only operate on a list of numbers (integers, decimals, bytes) and tables.
Currently the following functions are implemented:

* `math abs`: Returns absolute values of a list of numbers
* `math avg`: Finds the average of a list of numbers or tables
* `math ceil`: Applies the ceil function to a list of numbers
* [`math eval`](math-eval.md): Evaluates a list of math expressions into numbers
* `math floor`: Applies the floor function to a list of numbers
* `math max`: Finds the maximum within a list of numbers or tables
* `math median`: Finds the median of a list of numbers or tables
* `math min`: Finds the minimum within a list of numbers or tables
* `math mode`: Finds the most frequent element(s) within a list of numbers or tables
* `math round`: Applies the round function to a list of numbers
* `math sqrt`: Applies the square root function to a list of numbers
* `math stddev`: Finds the standard deviation of a list of numbers or tables
* `math sum`: Finds the sum of a list of numbers or tables
* `math product`: Finds the product of a list of numbers or tables
* `math variance`: Finds the variance of a list of numbers or tables

However, the mathematical functions like `min` and `max` are more permissive and also work on `Dates`.

## Examples

To get the average of the file sizes in a directory, simply pipe the size column from the ls command to the average command.

### List of Numbers (Integers, Decimals, Bytes)

```shell
> ls
 #  │ name               │ type │ size     │ modified
────┼────────────────────┼──────┼──────────┼─────────────
  0 │ CODE_OF_CONDUCT.md │ File │   3.4 KB │ 4 days ago
  1 │ CONTRIBUTING.md    │ File │   1.3 KB │ 4 days ago
  2 │ Cargo.lock         │ File │ 106.3 KB │ 6 mins ago
  3 │ Cargo.toml         │ File │   4.6 KB │ 3 days ago
  4 │ LICENSE            │ File │   1.1 KB │ 4 days ago
  5 │ Makefile.toml      │ File │    449 B │ 4 days ago
  6 │ README.md          │ File │  16.0 KB │ 6 mins ago
  7 │ TODO.md            │ File │      0 B │ 6 mins ago
  8 │ assets             │ Dir  │    128 B │ 4 days ago
  9 │ build.rs           │ File │     78 B │ 4 days ago
 10 │ crates             │ Dir  │    672 B │ 3 days ago
 11 │ debian             │ Dir  │    352 B │ 4 days ago
 12 │ docker             │ Dir  │    288 B │ 4 days ago
 13 │ docs               │ Dir  │    160 B │ 4 days ago
 14 │ features.toml      │ File │    632 B │ 4 days ago
 15 │ images             │ Dir  │    160 B │ 4 days ago
 16 │ justfile           │ File │    234 B │ 3 days ago
 17 │ rustfmt.toml       │ File │     16 B │ 4 days ago
 18 │ src                │ Dir  │    128 B │ 4 days ago
 19 │ target             │ Dir  │    192 B │ 8 hours ago
 20 │ tests              │ Dir  │    192 B │ 4 days ago
```

```shell
> ls | get size | math avg
───┬────────
 # │
───┼────────
 0 │ 7.2 KB
───┴────────
```

```shell
> ls | get size | math min
───┬─────
 # │
───┼─────
 0 │ 0 B
───┴─────
```

```shell
> ls | get size | math max
───┬──────────
 # │
───┼──────────
 0 │ 113.6 KB
───┴──────────
```

```shell
> ls | get size | math median
───┬───────
 # │
───┼───────
 0 │ 320 B
───┴───────
```

```shell
> ls | get size | math sum
───┬──────────
 # │
───┼──────────
 0 │ 143.6 KB
───┴──────────
```

```shell
> echo [3 3 9 12 12 15] | math mode
───┬────
 0 │  3
 1 │ 12
───┴────
```

```shell
> echo [2 3 3 4] | math product
72
```

```shell
> echo [1 4 6 10 50] | math stddev
18.1372
```

```shell
> echo [1 4 6 10 50] | math variance
328.96
```

```shell
> echo [1.5 2.3 -3.1] | math ceil
───┬────
 0 │  2
 1 │  3
 2 │ -3
───┴────
```

```shell
> echo [1.5 2.3 -3.1] | math floor
───┬────
 0 │  1
 1 │  2
 2 │ -4
───┴────
```

```shell
> echo [1.5 2.3 -3.1] | math round
───┬────
 0 │  2
 1 │  2
 2 │ -3
───┴────
```

```shell
> echo [4 16 0.25] | math sqrt
───┬────
 0 │  2
 1 │  4
 2 │  0.5
───┴────
```

```shell
> echo [1 -2 -3.0] | math abs
───┬────────
 0 │      1
 1 │      2
 2 │ 3.0000
───┴────────
```

### Dates

```shell
> ls | get modified | math min
2020-06-09 17:25:51.798743222 UTC
```

```shell
> ls | get modified | math max
2020-06-14 05:49:59.637449186 UT
```

### Operations on tables

```shell
>  pwd | split row / | size
───┬───────┬───────┬───────┬────────────
 # │ lines │ words │ chars │ bytes
───┼───────┼───────┼───────┼────────────
 0 │     0 │     1 │     5 │          5
 1 │     0 │     1 │    11 │         11
 2 │     0 │     1 │    11 │         11
 3 │     0 │     1 │     4 │          4
 4 │     0 │     2 │    12 │         12
 5 │     0 │     1 │     7 │          7
───┴───────┴───────┴───────┴────────────
```

```shell
> pwd | split row / | size | math max
────────────┬────
 lines      │ 0
 words      │ 2
 chars      │ 12
 bytes │ 12
────────────┴────
```

```shell
> pwd | split row / | size | math avg
────────────┬────────
 lines      │ 0.0000
 words      │ 1.1666
 chars      │ 8.3333
 bytes │ 8.3333
────────────┴────────
```

To get the sum of the characters that make up your present working directory.

```shell
> pwd | split row / | size | get chars | math sum
50
```

## Errors

`math` functions are aggregation functions so empty lists are invalid

```shell
> echo [] | math avg
error: Error: Unexpected: Cannot perform aggregate math operation on empty data
```
