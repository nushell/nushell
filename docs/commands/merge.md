# merge
Merge a table.

## Usage
```shell
> merge <block> {flags} 
 ```

## Parameters
* `<block>` the block to run and merge into the table

## Flags
* -h, --help: Display this help message

## Examples
  Merge a 1-based index column with some ls output
```shell
> ls | select name | keep 3 | merge { echo [1 2 3] | wrap index }
 ```

