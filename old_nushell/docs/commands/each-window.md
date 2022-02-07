# each window
Runs a block on sliding windows of `window_size` rows of a table at a time.

## Usage
```shell
> each window <window_size> <block> {flags} 
 ```

## Parameters
* `<window_size>` the size of each window
* `<block>` the block to run on each group

## Flags
* -h, --help: Display this help message
* -s, --stride <integer>: the number of rows to slide over between windows

## Examples
  Echo the sum of each window
```shell
> echo [1 2 3 4] | each window 2 { echo $it | math sum }
 ```

