# do
Runs a block, optionally ignoring errors.

## Usage
```shell
> do <block> ...args {flags} 
 ```

## Parameters
* `<block>` the block to run 
* ...args: the parameter(s) for the block

## Flags
* -h, --help: Display this help message
* -i, --ignore-errors: ignore errors as the block runs

## Examples
  Run the block
```shell
> do { echo hello }
 ```

  Run the block and ignore errors
```shell
> do -i { thisisnotarealcommand }
 ```

  Run the block with a parameter
```shell
> do { |x| $x + 100 } 55
 ```

