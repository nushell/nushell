# reduce
Aggregate a list table to a single value using an accumulator block.

Block must be (A, A) -> A unless --fold is selected, in which case it may be A, B -> A.

## Usage
```shell
> reduce <block> {flags} 
 ```

## Parameters
* `<block>` reducing function

## Flags
* -h, --help: Display this help message
* -f, --fold <any>: reduce with initial value
* -n, --numbered: returned a numbered item ($it.index and $it.item)

## Examples
  Simple summation (equivalent to math sum)
```shell
> echo 1 2 3 4 | reduce { $acc + $it }
 ```

  Summation from starting value using fold
```shell
> echo 1 2 3 4 | reduce -f (-1) { $acc + $it }
 ```

  Folding with rows
```shell
> <table> | reduce -f 1.6 { $acc * (echo $it.a | str to-int) + (echo $it.b | str to-int) }
 ```

  Numbered reduce to find index of longest word
```shell
> echo one longest three bar | reduce -n { if ($it.item | str length) > ($acc.item | str length) {echo $it} {echo $acc}} | get index
 ```

