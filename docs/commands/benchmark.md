# benchmark
Runs a block and returns the time it took to execute it.

## Usage
```shell
> benchmark <block> {flags} 
 ```

## Parameters
* `<block>` the block to run and benchmark

## Flags
* -h, --help: Display this help message
* -p, --passthrough <block>: Display the benchmark results and pass through the block's output

## Examples
  Benchmarks a command within a block
```shell
> benchmark { sleep 500ms }
 ```

  Benchmarks a command within a block and passes its output through
```shell
> echo 45 | benchmark { sleep 500ms } --passthrough {}
 ```

