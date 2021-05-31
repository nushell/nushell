# for
Run a block on each row of the table.

## Usage
```shell
> for <var> <in> <value> <block> {flags} 
 ```

## Parameters
* `<var>` the name of the variable
* `<in>` the word 'in'
* `<value>` the value we want to iterate
* `<block>` the block to run on each item

## Flags
* -h, --help: Display this help message
* -n, --numbered: returned a numbered item ($it.index and $it.item)

## Examples
  Echo the square of each integer
```shell
> for x in [1 2 3] { $x * $x }
 ```

  Work with elements of a range
```shell
> for $x in 1..3 { $x }
 ```

  Number each item and echo a message
```shell
> for $it in ['bob' 'fred'] --numbered { $"($it.index) is ($it.item)" }
 ```

