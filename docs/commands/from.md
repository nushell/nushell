# from csv

Converts content (string or binary) into a table. The source format is specified as a subcommand, like `from csv` or `from json`.

Use this when nushell cannot determine the input file extension.

## Available Subcommands

* from bson
* [from csv](from-csv.md)
* from eml
* [from ics](from-ics.md)
* [from ini](from-ini.md)
* [from json](from-json.md)
* [from ods](from-ods.md)
* from sqlite
* from ssv
* [from toml](from-toml.md)
* [from tsv](from-tsv.md)
* [from url](from-url.md)
* [from vcf](from-vcf.md)
* [from xlsx](from-xlsx.md)
* [from xml](from-xml.md)
* [from yaml](from-yaml.md)

*Subcommands without links are currently missing their documentation.*

## Example for `from csv`

Let's say we have the following file:

```shell
> cat pets.txt
animal, name, age
cat, Tom, 7
dog, Alfred, 10
chameleon, Linda, 1
```

`pets.txt` is actually a .csv file but it has the .txt extension, `open` is not able to convert it into a table:

```shell
> open pets.txt
animal, name, age
cat, Tom, 7
dog, Alfred, 10
chameleon, Linda, 1
```

To get a table from `pets.txt` we need to use the `from csv` command:

```shell
> open pets.txt | from csv
━━━┯━━━━━━━━━━━┯━━━━━━━━━┯━━━━━━
 # │ animal    │  name   │  age
───┼───────────┼─────────┼──────
 0 │ cat       │  Tom    │  7
 1 │ dog       │  Alfred │  10
 2 │ chameleon │  Linda  │  1
━━━┷━━━━━━━━━━━┷━━━━━━━━━┷━━━━━━
```
