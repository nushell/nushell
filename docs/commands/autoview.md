# autoview

Print the content of the pipeline as a table or list.
It is the implied or default viewer when none is provided.

When reading a single value, a table or a list, `autoview` will attempt to view it.
When reading a string that originally comes from a source file it will attempt
to use `textview`.
When reading a binary file it will attempt to display its content as hexadecimal
numbers and the corresponding characters.

`-h`, `--help`
  Display help message.

## Examples

In all following examples `autoview` can be removed with no change in the output.
The use of `autoview` at the end of the pipeline is implied when no viewer is
explicitly used.

```shell
> which nu | get path | autoview
/home/me/.cargo/bin/nu
```

```shell
> ls | autoview
────┬────────────────────┬──────┬─────────┬──────────────
 #  │ name               │ type │ size    │ modified
────┼────────────────────┼──────┼─────────┼──────────────
  0 │ README.md          │ File │   932 B │ 19 hours ago
  1 │ alias.md           │ File │  2.0 KB │ 19 hours ago
  2 │ append.md          │ File │  1.4 KB │ 19 hours ago
   ...
 82 │ wrap.md            │ File │  1.8 KB │ 19 hours ago
────┴────────────────────┴──────┴─────────┴──────────────
```

```shell
> echo "# Hi" "## Section" "Some text" | save file.md
> open file.md | autoview
# Hi
## Section
Some text
```

`autoview` will use `textview` to colorize the text based on the file format.
The style used by `textview` can be configured in `config.toml`.

```shell
> open --raw (which nu | get path) | autoview
...
126d1c0:   64 31 66 37  62 30 31 63  36 2e 31 31  38 2e 6c 6c   d1f7b01c6.118.ll
126d1d0:   76 6d 2e 34  34 38 37 35  37 31 32 34  39 35 33 39   vm.4487571249539
126d1e0:   34 34 30 34  30 39 00 61  6e 6f 6e 2e  30 30 61 63   440409.anon.00ac
126d1f0:   37 32 65 36  37 66 32 31  39 34 62 32  32 61 61 63   72e67f2194b22aac
126d200:   62 35 39 37  33 36 30 62  64 31 39 38  2e 31 36 2e   b597360bd198.16.
...
```
