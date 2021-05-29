# group-by date
creates a table grouped by date.

## Usage
```shell
> group-by date (column_name) {flags} 
 ```

## Parameters
* `(column_name)` the name of the column to group by

## Flags
* -h, --help: Display this help message
* -f, --format <string>: Specify date and time formatting

## Examples
  Group files by type
```shell
> ls | group-by date --format '%d/%m/%Y'
 ```

