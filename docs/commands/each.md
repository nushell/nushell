# each
Run a block on each row of the table.

## Usage
```shell
> each <block> <subcommand> {flags} 
 ```

## Subcommands
* each group - Runs a block on groups of `group_size` rows of a table at a time.
* each window - Runs a block on sliding windows of `window_size` rows of a table at a time.

## Parameters
* `<block>` the block to run on each row

## Flags
* -h, --help: Display this help message
* -n, --numbered: returned a numbered item ($it.index and $it.item)

## Examples
  Echo the sum of each row
```shell
> echo [[1 2] [3 4]] | each { echo $it | math sum }
 ```

  Echo the square of each integer
```shell
> echo [1 2 3] | each { echo ($it * $it) }
 ```

  Number each item and echo a message
```shell
> echo ['bob' 'fred'] | each --numbered { echo $"($it.index) is ($it.item)" }
 ```

  Name the block variable that each uses
```shell
> [1, 2, 3] | each {|x| $x + 100}
 ```

