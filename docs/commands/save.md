# save

This command saves the contents of the pipeline to a file. Use this in combination with the `to-json`, `to-csv`, ... commands to save the contents in the specified format.

Syntax: `save (path) {flags}`

### Parameters:

* `(path)` the path to save contents to

### Flags

    --raw
      treat values as-is rather than auto-converting based on file extension

## Example

You can save the name of files in a directory like this:

```shell
> ls | where type == File | pick name | save filenames.csv
```

Or you can format it in supported formats using one of the `to-*` commands:

```shell
> ls | where type == File | pick name | to-csv | save filenames
```

`filename.csv` and `filenames` are both `csv` formatted files. Nu auto-converts the format if a supported file extension is given.