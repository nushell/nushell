# drop
Remove the last number of rows or columns.

## Usage
```shell
> drop (rows) <subcommand> {flags} 
 ```

## Subcommands
* drop column - Remove the last number of columns. If you want to remove columns by name, try 'reject'.

## Parameters
* `(rows)` starting from the back, the number of rows to remove

## Flags
* -h, --help: Display this help message

## Examples
  Remove the last item of a list/table
```shell
> echo [1 2 3] | drop
 ```

  Remove the last 2 items of a list/table
```shell
> echo [1 2 3] | drop 2
 ```

