# drop column
Remove the last number of columns. If you want to remove columns by name, try 'reject'.

## Usage
```shell
> drop column (columns) {flags} 
 ```

## Parameters
* `(columns)` starting from the end, the number of columns to remove

## Flags
* -h, --help: Display this help message

## Examples
  Remove the last column of a table
```shell
> echo [[lib, extension]; [nu-lib, rs] [nu-core, rb]] | drop column
 ```

