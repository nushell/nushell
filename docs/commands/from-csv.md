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

