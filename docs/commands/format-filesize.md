# format filesize
Converts a column of filesizes to some specified format

## Usage
```shell
> format filesize <field> <format value> {flags} 
 ```

## Parameters
* `<field>` the name of the column to update
  <format value> the format into which convert the filesizes

## Flags
* -h, --help: Display this help message

## Examples
  Convert the size row to KB
```shell
> ls | format filesize size KB
 ```

  Convert the apparent row to B
```shell
> du | format filesize apparent B
 ```

