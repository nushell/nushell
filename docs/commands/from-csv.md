# from-csv

Converts csv data into table. Use this when nushell cannot dertermine the input file extension.

## Example

Let's say we have the following file :

```shell
> cat pets.txt
animal, name, age
cat, Tom, 7
dog, Alfred, 10
chameleon, Linda, 1
```

`pets.txt` is actually a .csv file but it has the .txt extension, `open` is not able to convert it into a table :

```shell
> open pets.txt
animal, name, age
cat, Tom, 7
dog, Alfred, 10
chameleon, Linda, 1
```

To get a table from `pets.txt` we need to use the `from-csv` command :

```shell
> open pets.txt | from-csv
━━━┯━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━
 # │ animal    │  name   │  age
───┼───────────┼─────────┼──────
 0 │ cat       │  Tom    │  7
 1 │ dog       │  Alfred │  10
 2 │ chameleon │  Linda  │  1
━━━┷━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━
```

To ignore the csv headers use `--headerless` :

```shell
━━━┯━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━━━━
 # │ Column1   │ Column2 │ Column3
───┼───────────┼─────────┼─────────
 0 │ dog       │  Alfred │  10
 1 │ chameleon │  Linda  │  1
━━━┷━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━━━━
```

To split on a character other than ',' use `--separator` :

```shell
> open pets.txt
animal; name; age
cat; Tom; 7
dog; Alfred; 10
chameleon; Linda; 1
```

```shell
> open pets.txt | from-csv --separator ';'
━━━┯━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━
 # │ animal    │  name   │  age
───┼───────────┼─────────┼──────
 0 │ cat       │  Tom    │  7
 1 │ dog       │  Alfred │  10
 2 │ chameleon │  Linda  │  1
━━━┷━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━
```

To use this command to open a csv with separators other than a comma, use the `--raw` switch of `open` to open the csv, othewise the csv will enter `from-csv` as a table split on commas rather than raw text.

```shell
> mv pets.txt pets.csv
> open pets.csv | from-csv --separator ';'
error: Expected a string from pipeline
- shell:1:16
1 | open pets.csv | from-csv --separator ';'
  |                 ^^^^^^^^ requires string input
- shell:1:0
1 | open pets.csv | from-csv --separator ';'
  |  value originates from here

> open pets.csv --raw | from-csv --separator ';'
━━━┯━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━
 # │ animal    │  name   │  age
───┼───────────┼─────────┼──────
 0 │ cat       │  Tom    │  7
 1 │ dog       │  Alfred │  10
 2 │ chameleon │  Linda  │  1
━━━┷━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━
```

The string '\t' can be used to separate on tabs. Note that this is the same as using the from-tsv command.

Newlines '\n' are not acceptable separators.

Note that separators are currently provided as strings and need to be wrapped in quotes.

```shell
> open pets.csv --raw | from-csv --separator ;
- shell:1:43
1 | open pets.csv --raw | from-csv --separator ;
  |                                            ^
```

It is also considered an error to use a separator greater than one char :

```shell
> open pets.txt | from-csv --separator '123'
error: Expected a single separator char from --separator
- shell:1:37
1 | open pets.txt | from-csv --separator '123'
  |                                      ^^^^^ requires a single character string input
```
