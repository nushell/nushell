# each group
Runs a block on groups of `group_size` rows of a table at a time.

## Usage
```shell
> each group <group_size> <block> {flags} 
 ```

## Parameters
* `<group_size>` the size of each group
* `<block>` the block to run on each group

## Flags
* -h, --help: Display this help message

## Examples
  Echo the sum of each pair
```shell
> echo [1 2 3 4] | each group 2 { echo $it | math sum }
 ```

