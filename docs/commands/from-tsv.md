# from-tsv

Parse text as `.tsv` and create table.

Syntax: `from-tsv {flags}`

### Flags:

    --headerless
      don't treat the first row as column names

## Examples

Let's say we have the following file which is formatted like a `tsv` file:

```shell
> open elements.txt
Symbol        Element
H        Hydrogen
He        Helium
Li        Lithium
Be        Beryllium
```

If we pass the output of the `open` command to `from-tsv` we get a correct formatted table:

```shell
> open elements.txt | from-tsv
━━━┯━━━━━━━━┯━━━━━━━━━━━
 # │ Symbol │ Element
───┼────────┼───────────
 0 │ H      │ Hydrogen
 1 │ He     │ Helium
 2 │ Li     │ Lithium
 3 │ Be     │ Beryllium
━━━┷━━━━━━━━┷━━━━━━━━━━━
```

Using the `--headerless` flag has the following output:

```shell
> open elements.txt | from-tsv --headerless
━━━━┯━━━━━━━━━┯━━━━━━━━━━━
 #  │ Column1 │ Column2
────┼─────────┼───────────
  0 │ Symbol  │ Element
  1 │ H       │ Hydrogen
  2 │ He      │ Helium
  3 │ Li      │ Lithium
  4 │ Be      │ Beryllium
━━━━┷━━━━━━━━━┷━━━━━━━━━━━
```